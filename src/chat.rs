use crate::schema::{
    ComplexityLevel, ExpertiseLevel, NotificationLevel, SchemaEvolutionEngine, UserContext,
    UserPreferences,
};
use crate::utir::Bits;
use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

/// The Generative Chat Interface - where users sculpt consciousness through conversation
#[derive(Clone)]
pub struct GenerativeChatEngine {
    pub schema_engine: Arc<Mutex<SchemaEvolutionEngine>>,
    pub active_sessions: Arc<RwLock<HashMap<Uuid, ChatSession>>>,
    pub chat_rules: ChatRules,
}

#[derive(Debug, Clone)]
pub struct ChatSession {
    pub session_id: Uuid,
    pub user_context: UserContext,
    pub conversation_thread: Uuid,
    pub current_schema_version: String,
    pub pending_changes: Vec<PendingChange>,
    pub chat_state: ChatState,
    pub websocket_connected: bool,
}

#[derive(Debug, Clone)]
pub struct PendingChange {
    pub change_id: Uuid,
    pub description: String,
    pub impact_level: ImpactLevel,
    pub approval_required: bool,
    pub auto_apply_in_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Minimal,     // Safe changes, auto-apply
    Moderate,    // Preview required
    Significant, // Approval required
    Breaking,    // Special handling required
}

#[derive(Debug, Clone)]
pub enum ChatState {
    Exploring,     // User is learning about current API
    Designing,     // User is proposing changes
    Reviewing,     // User is reviewing proposed changes
    Implementing,  // Changes are being applied
    Testing,       // User is testing new schema version
    Collaborating, // Multiple users working together
}

/// Chat interface configuration and rules
#[derive(Debug, Clone)]
pub struct ChatRules {
    pub max_concurrent_changes: u32,
    pub auto_apply_safe_changes: bool,
    pub require_confirmation_for: Vec<String>,
    pub conversation_timeout_minutes: u64,
    pub real_time_preview_enabled: bool,
}

/// Incoming chat messages from users
#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub content: String,
    pub message_type: MessageType,
    pub context: Option<ChatContext>,
}

#[derive(Debug, Deserialize)]
pub enum MessageType {
    Query,         // User asking about current API
    Suggestion,    // User suggesting a change
    Approval,      // User approving a pending change
    Rejection,     // User rejecting a pending change
    Exploration,   // User wants to explore possibilities
    Collaboration, // User wants to work with others
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatContext {
    pub referring_to_endpoint: Option<String>,
    pub referring_to_schema: Option<String>,
    pub urgency_level: Option<UrgencyLevel>,
    pub collaboration_mode: Option<CollaborationMode>,
}

#[derive(Debug, Deserialize)]
pub enum UrgencyLevel {
    Low,      // Can wait for next version
    Medium,   // Should be in next minor version
    High,     // Should be in next patch version
    Critical, // Needs immediate attention
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CollaborationMode {
    Solo,          // Just this user
    TeamReview,    // Share with team for input
    PublicRFC,     // Open for community input
    ExpertConsult, // Escalate to schema experts
}

/// Outgoing responses from the chat system
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub response_type: ResponseType,
    pub schema_changes: Option<SchemaChangePreview>,
    pub bits: Bits,
    pub actions_available: Vec<AvailableAction>,
    pub conversation_state: ConversationState,
}

#[derive(Debug, Serialize)]
pub enum ResponseType {
    Information,  // Providing info about current API
    Confirmation, // Confirming understanding of request
    Preview,      // Showing what changes would look like
    Applied,      // Changes have been applied
    Warning,      // Potential issues with request
    Error,        // Request cannot be fulfilled
    Suggestion,   // AI suggesting alternative approach
}

#[derive(Debug, Serialize)]
pub struct SchemaChangePreview {
    pub new_version: String,
    pub changes_summary: Vec<String>,
    pub breaking_changes: Vec<String>,
    pub migration_notes: Vec<String>,
    pub preview_url: String,
    pub estimated_impact: ImpactAnalysis,
}

#[derive(Debug, Serialize)]
pub struct ImpactAnalysis {
    pub affected_endpoints: u32,
    pub new_capabilities: Vec<String>,
    pub deprecated_features: Vec<String>,
    pub performance_impact: String,
    pub client_migration_effort: String,
}

#[derive(Debug, Serialize)]
pub struct AvailableAction {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub risk_level: ImpactLevel,
    pub keyboard_shortcut: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConversationState {
    pub session_id: Uuid,
    pub messages_in_conversation: u32,
    pub pending_changes_count: u32,
    pub current_schema_version: String,
    pub conversation_focus: String, // What aspect of API we're discussing
    pub suggested_next_steps: Vec<String>,
}

/// WebSocket message types for real-time collaboration
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "chat")]
    Chat {
        content: String,
        context: Option<ChatContext>,
    },

    #[serde(rename = "schema_preview")]
    SchemaPreview { version: String },

    #[serde(rename = "approve_change")]
    ApproveChange { change_id: Uuid },

    #[serde(rename = "reject_change")]
    RejectChange {
        change_id: Uuid,
        reason: Option<String>,
    },

    #[serde(rename = "collaborate")]
    Collaborate {
        invite_users: Vec<String>,
        mode: CollaborationMode,
    },

    #[serde(rename = "heartbeat")]
    Heartbeat,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WebSocketResponse {
    #[serde(rename = "message")]
    Message(ChatResponse),

    #[serde(rename = "schema_updated")]
    SchemaUpdated {
        new_version: String,
        changes: Vec<String>,
        live_preview_url: String,
    },

    #[serde(rename = "collaboration_started")]
    CollaborationStarted {
        session_id: Uuid,
        participants: Vec<String>,
        shared_workspace_url: String,
    },

    #[serde(rename = "change_applied")]
    ChangeApplied {
        change_id: Uuid,
        new_schema_version: String,
        rollback_token: String,
    },

    #[serde(rename = "error")]
    Error { message: String, code: String },

    #[serde(rename = "heartbeat")]
    Heartbeat,
}

impl GenerativeChatEngine {
    pub fn new(schema_engine: SchemaEvolutionEngine) -> Self {
        Self {
            schema_engine: Arc::new(Mutex::new(schema_engine)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            chat_rules: ChatRules::default(),
        }
    }

    /// Start a new chat session
    pub async fn start_session(&self, user_context: UserContext) -> Result<Uuid> {
        let session_id = Uuid::new_v4();

        // Start conversation thread in schema engine
        let conversation_thread = {
            let mut schema_engine = self.schema_engine.lock().await;
            schema_engine.start_conversation(user_context.clone())
        };

        let session = ChatSession {
            session_id,
            user_context,
            conversation_thread,
            current_schema_version: "1.0.0".to_string(), // Get from schema engine
            pending_changes: Vec::new(),
            chat_state: ChatState::Exploring,
            websocket_connected: false,
        };

        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session_id, session);

        info!("Started new chat session: {}", session_id);
        Ok(session_id)
    }

    /// Process a chat message and generate response
    pub async fn process_message(
        &self,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<ChatResponse> {
        let mut sessions = self.active_sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Update chat state based on message type
        self.update_chat_state(session, &message);

        // Process through schema evolution engine
        let schema_response = {
            let mut schema_engine = self.schema_engine.lock().await;
            schema_engine
                .process_message(session.conversation_thread, message.content.clone())
                .await?
        };

        // Generate chat response based on schema evolution response
        let chat_response = self.generate_chat_response(&schema_response, session, &message)?;

        // Update session state
        if let Some(new_version) = &schema_response.new_schema_version {
            session.current_schema_version = new_version.clone();
        }

        Ok(chat_response)
    }

    fn update_chat_state(&self, session: &mut ChatSession, message: &ChatMessage) {
        session.chat_state = match message.message_type {
            MessageType::Query => ChatState::Exploring,
            MessageType::Suggestion => ChatState::Designing,
            MessageType::Approval | MessageType::Rejection => ChatState::Reviewing,
            MessageType::Exploration => ChatState::Exploring,
            MessageType::Collaboration => ChatState::Collaborating,
        };
    }

    fn generate_chat_response(
        &self,
        schema_response: &crate::schema::SchemaEvolutionResponse,
        session: &ChatSession,
        original_message: &ChatMessage,
    ) -> Result<ChatResponse> {
        let response_type = if schema_response.mutations_applied > 0 {
            ResponseType::Applied
        } else if schema_response.explanation.contains("understand") {
            ResponseType::Information
        } else {
            ResponseType::Confirmation
        };

        let schema_changes = if let Some(version) = &schema_response.new_schema_version {
            Some(SchemaChangePreview {
                new_version: version.clone(),
                changes_summary: vec![schema_response.explanation.clone()],
                breaking_changes: vec![], // Would analyze from mutations
                migration_notes: vec!["No migration required".to_string()],
                preview_url: format!("/schema/{}/preview", version),
                estimated_impact: ImpactAnalysis {
                    affected_endpoints: schema_response.mutations_applied,
                    new_capabilities: vec!["Enhanced API surface".to_string()],
                    deprecated_features: vec![],
                    performance_impact: "Minimal".to_string(),
                    client_migration_effort: "None".to_string(),
                },
            })
        } else {
            None
        };

        let actions_available = match session.chat_state {
            ChatState::Exploring => vec![
                AvailableAction {
                    action_id: "suggest_change".to_string(),
                    label: "Suggest a change".to_string(),
                    description: "Propose modifications to the API".to_string(),
                    risk_level: ImpactLevel::Minimal,
                    keyboard_shortcut: Some("s".to_string()),
                },
                AvailableAction {
                    action_id: "explore_endpoints".to_string(),
                    label: "Explore existing endpoints".to_string(),
                    description: "Learn about current API capabilities".to_string(),
                    risk_level: ImpactLevel::Minimal,
                    keyboard_shortcut: Some("e".to_string()),
                },
            ],
            ChatState::Designing => vec![
                AvailableAction {
                    action_id: "preview_changes".to_string(),
                    label: "Preview changes".to_string(),
                    description: "See how your suggestions would look".to_string(),
                    risk_level: ImpactLevel::Moderate,
                    keyboard_shortcut: Some("p".to_string()),
                },
                AvailableAction {
                    action_id: "apply_changes".to_string(),
                    label: "Apply changes".to_string(),
                    description: "Create new schema version with your changes".to_string(),
                    risk_level: ImpactLevel::Significant,
                    keyboard_shortcut: Some("a".to_string()),
                },
            ],
            _ => vec![],
        };

        Ok(ChatResponse {
            message: self.enhance_response_message(&schema_response.explanation, session)?,
            response_type,
            schema_changes,
            bits: schema_response.bits.clone(),
            actions_available,
            conversation_state: ConversationState {
                session_id: session.session_id,
                messages_in_conversation: 1, // Would track from conversation memory
                pending_changes_count: session.pending_changes.len() as u32,
                current_schema_version: session.current_schema_version.clone(),
                conversation_focus: self.determine_conversation_focus(original_message),
                suggested_next_steps: self.suggest_next_steps(session),
            },
        })
    }

    fn enhance_response_message(
        &self,
        base_message: &str,
        session: &ChatSession,
    ) -> Result<String> {
        let enhancement = match session.user_context.expertise_level {
            ExpertiseLevel::Beginner => {
                "\n\n💡 **Tip**: You can ask me to explain any technical terms or walk you through the changes step by step."
            },
            ExpertiseLevel::Intermediate => {
                "\n\n🔧 **Next**: You can preview these changes or ask me to suggest related improvements."
            },
            ExpertiseLevel::Expert => {
                "\n\n⚡ **Advanced**: Use `/schema diff` to see detailed changes or `/rollback` if needed."
            },
            ExpertiseLevel::Architect => {
                "\n\n🏗️ **Architect Mode**: Full schema modification capabilities available. Pattern analysis and evolution rules can be customized."
            },
        };

        Ok(format!("{}{}", base_message, enhancement))
    }

    fn determine_conversation_focus(&self, message: &ChatMessage) -> String {
        if message.content.contains("endpoint") {
            "API Endpoints".to_string()
        } else if message.content.contains("response") || message.content.contains("field") {
            "Response Schema".to_string()
        } else if message.content.contains("validation") || message.content.contains("error") {
            "Validation & Error Handling".to_string()
        } else {
            "General API Design".to_string()
        }
    }

    fn suggest_next_steps(&self, session: &ChatSession) -> Vec<String> {
        match session.chat_state {
            ChatState::Exploring => vec![
                "Ask about specific endpoints you'd like to understand".to_string(),
                "Suggest improvements to existing functionality".to_string(),
                "Explore what new capabilities you'd like to add".to_string(),
            ],
            ChatState::Designing => vec![
                "Preview your proposed changes".to_string(),
                "Consider backward compatibility implications".to_string(),
                "Test the changes in a sandbox environment".to_string(),
            ],
            ChatState::Reviewing => vec![
                "Approve or reject pending changes".to_string(),
                "Request modifications to proposed changes".to_string(),
                "Collaborate with team members for feedback".to_string(),
            ],
            _ => vec!["Continue the conversation".to_string()],
        }
    }

    /// Handle WebSocket connection for real-time collaboration
    pub async fn handle_websocket(&self, session_id: Uuid, websocket: WebSocket) {
        info!("WebSocket connected for session: {}", session_id);

        // Update session to mark WebSocket as connected
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.websocket_connected = true;
            }
        }

        let (mut sender, mut receiver) = websocket.split();

        // Handle incoming WebSocket messages
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(ws_message) => {
                        if let Ok(response) =
                            self.handle_websocket_message(session_id, ws_message).await
                        {
                            let response_json =
                                serde_json::to_string(&response).unwrap_or_default();
                            if sender.send(Message::Text(response_json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Invalid WebSocket message: {}", e);
                        let error_response = WebSocketResponse::Error {
                            message: "Invalid message format".to_string(),
                            code: "INVALID_FORMAT".to_string(),
                        };
                        let error_json = serde_json::to_string(&error_response).unwrap_or_default();
                        if sender.send(Message::Text(error_json)).await.is_err() {
                            break;
                        }
                    }
                },
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed for session: {}", session_id);
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error for session {}: {}", session_id, e);
                    break;
                }
                _ => {}
            }
        }

        // Mark WebSocket as disconnected
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.websocket_connected = false;
            }
        }
    }

    async fn handle_websocket_message(
        &self,
        session_id: Uuid,
        message: WebSocketMessage,
    ) -> Result<WebSocketResponse> {
        match message {
            WebSocketMessage::Chat { content, context } => {
                let chat_message = ChatMessage {
                    content,
                    message_type: MessageType::Query, // Default type
                    context,
                };

                let chat_response = self.process_message(session_id, chat_message).await?;
                Ok(WebSocketResponse::Message(chat_response))
            }

            WebSocketMessage::ApproveChange { change_id } => {
                // Handle change approval
                Ok(WebSocketResponse::ChangeApplied {
                    change_id,
                    new_schema_version: "1.0.1".to_string(), // Would get from actual application
                    rollback_token: Uuid::new_v4().to_string(),
                })
            }

            WebSocketMessage::Heartbeat => Ok(WebSocketResponse::Heartbeat),

            _ => Ok(WebSocketResponse::Error {
                message: "Message type not yet implemented".to_string(),
                code: "NOT_IMPLEMENTED".to_string(),
            }),
        }
    }
}

impl Default for ChatRules {
    fn default() -> Self {
        Self {
            max_concurrent_changes: 5,
            auto_apply_safe_changes: true,
            require_confirmation_for: vec![
                "DELETE".to_string(),
                "BREAKING_CHANGE".to_string(),
                "DEPRECATE".to_string(),
            ],
            conversation_timeout_minutes: 60,
            real_time_preview_enabled: true,
        }
    }
}

/// Create chat router for the API
pub fn create_chat_router(chat_engine: GenerativeChatEngine) -> Router {
    Router::new()
        .route("/chat/sessions", post(start_chat_session))
        .route(
            "/chat/sessions/:session_id/messages",
            post(send_chat_message),
        )
        .route("/chat/sessions/:session_id/ws", get(websocket_handler))
        .route(
            "/chat/sessions/:session_id/schema/preview",
            get(get_schema_preview),
        )
        .route(
            "/chat/sessions/:session_id/changes/:change_id/approve",
            post(approve_change),
        )
        .with_state(chat_engine)
}

/// Start a new chat session
async fn start_chat_session(
    State(chat_engine): State<GenerativeChatEngine>,
    Json(user_context): Json<UserContext>,
) -> Result<Json<ChatSessionResponse>, axum::http::StatusCode> {
    match chat_engine.start_session(user_context).await {
        Ok(session_id) => Ok(Json(ChatSessionResponse {
            session_id,
            websocket_url: format!("/chat/sessions/{}/ws", session_id),
            schema_preview_url: format!("/chat/sessions/{}/schema/preview", session_id),
            status: "active".to_string(),
        })),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Send a message to a chat session
async fn send_chat_message(
    State(chat_engine): State<GenerativeChatEngine>,
    Path(session_id): Path<Uuid>,
    Json(message): Json<ChatMessage>,
) -> Result<Json<ChatResponse>, axum::http::StatusCode> {
    match chat_engine.process_message(session_id, message).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// WebSocket handler for real-time collaboration
async fn websocket_handler(
    State(chat_engine): State<GenerativeChatEngine>,
    Path(session_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |websocket| chat_engine.handle_websocket(session_id, websocket))
}

/// Get schema preview for current session
async fn get_schema_preview(
    State(_chat_engine): State<GenerativeChatEngine>,
    Path(_session_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    // Would return current schema version being worked on
    Json(serde_json::json!({
        "message": "Schema preview endpoint - implementation needed",
        "version": "1.0.1-preview"
    }))
}

/// Approve a pending change
async fn approve_change(
    State(_chat_engine): State<GenerativeChatEngine>,
    Path((_session_id, _change_id)): Path<(Uuid, Uuid)>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "approved",
        "applied": true
    }))
}

#[derive(Debug, Serialize)]
struct ChatSessionResponse {
    session_id: Uuid,
    websocket_url: String,
    schema_preview_url: String,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaEvolutionEngine;

    #[tokio::test]
    async fn test_chat_session_creation() {
        let schema_engine = SchemaEvolutionEngine::new(create_test_schema());
        let chat_engine = GenerativeChatEngine::new(schema_engine);

        let user_context = UserContext {
            user_id: "test_user".to_string(),
            permissions: vec!["*".to_string()],
            preferences: UserPreferences {
                preferred_complexity: ComplexityLevel::Balanced,
                auto_apply_safe_changes: true,
                notification_preferences: NotificationLevel::Important,
            },
            expertise_level: ExpertiseLevel::Intermediate,
        };

        let session_id = chat_engine.start_session(user_context).await.unwrap();
        assert!(!session_id.is_nil());

        let sessions = chat_engine.active_sessions.read().await;
        assert!(sessions.contains_key(&session_id));
    }

    fn create_test_schema() -> crate::schema::LivingSchema {
        // Create a basic schema for testing
        crate::schema::LivingSchema {
            version: "1.0.0".to_string(),
            chat_thread_id: None,
            parent_version: None,
            created_by_conversation: None,
            openapi: crate::schema::OpenApiSpec {
                openapi: "3.0.0".to_string(),
                info: crate::schema::ApiInfo {
                    title: "Test API".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Test API for chat".to_string(),
                    consciousness_level: 1.0,
                    fractal_complexity: 1,
                },
                servers: vec![],
                paths: std::collections::BTreeMap::new(),
                components: crate::schema::Components {
                    schemas: std::collections::BTreeMap::new(),
                    responses: std::collections::BTreeMap::new(),
                    parameters: std::collections::BTreeMap::new(),
                },
                one_engine_extensions: crate::schema::OneEngineExtensions {
                    consciousness_patterns: vec![],
                    fractal_operations: crate::schema::FractalOperations {
                        encode_operations: vec![],
                        compile_operations: vec![],
                        execute_operations: vec![],
                        verify_operations: vec![],
                    },
                    evolution_rules: crate::schema::EvolutionRules {
                        max_endpoints_per_version: 100,
                        breaking_change_policy:
                            crate::schema::BreakingChangePolicy::RequireApproval,
                        auto_deprecation_rules: vec![],
                        merge_conflict_resolution: crate::schema::MergeStrategy::ConflictResolution,
                    },
                    chat_interface: crate::schema::ChatInterfaceConfig {
                        enabled: true,
                        schema_modification_permissions: vec!["*".to_string()],
                        approval_required_for: vec![],
                        real_time_preview: true,
                        version_branching_enabled: true,
                    },
                },
            },
            evolution_metadata: crate::schema::EvolutionMetadata {
                created_at: 0,
                conversation_messages: 0,
                schema_mutations: vec![],
                compatibility_analysis: crate::schema::CompatibilityAnalysis {
                    backward_compatible: true,
                    breaking_changes: vec![],
                    migration_complexity: crate::schema::MigrationComplexity::Trivial,
                    affected_clients: crate::schema::EstimatedImpact {
                        client_count_estimate: 0,
                        integration_complexity_increase: 0.0,
                        performance_impact_percentage: 0.0,
                    },
                },
                performance_impact: crate::schema::PerformanceImpact {
                    endpoint_count_delta: 0,
                    complexity_score_delta: 0.0,
                    estimated_response_time_change_ms: 0,
                    memory_usage_change_mb: 0,
                },
                user_satisfaction_score: None,
            },
        }
    }
}
