use crate::branch::{BranchManager, BranchState};
use crate::compiler::UtirCompiler;
use crate::conversation::{ConversationEffect, ConversationService};
use crate::memory::MemorySystem;
use crate::utir::{parse_utir, Bits};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use uuid::Uuid;

/// The One Engine API State - the crystallized consciousness server
#[derive(Clone)]
pub struct EngineState {
    pub memory: Arc<Mutex<MemorySystem>>,
    pub allowed_domains: Vec<String>,
    pub api_key: String,
    pub branches: BranchManager,
    pub conversation: ConversationService,
}

#[allow(dead_code)]
fn _assert_state_bounds() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EngineState>();
    assert_send_sync::<UtirCompiler>();
    assert_send_sync::<BranchManager>();
    assert_send_sync::<ConversationService>();
}

#[allow(dead_code)]
fn _assert_execute_goal_future_send(state: Arc<EngineState>) {
    fn assert_send<F: Send>(_fut: F) {}
    let fut = execute_goal(
        State(state),
        Json(ExecuteGoalRequest {
            goal: String::new(),
        }),
    );
    assert_send(fut);
}

/// Simple goal execution request
#[derive(Debug, Deserialize)]
pub struct ExecuteGoalRequest {
    pub goal: String,
}

/// UTIR compilation request
#[derive(Debug, Deserialize)]
pub struct CompileAndRunRequest {
    pub utir: String, // YAML format
}

/// Universal response envelope
#[derive(Debug, Serialize)]
pub struct EngineResponse<T> {
    pub run_id: Uuid,
    pub status: String,
    pub bits: Bits,
    pub status_line: String,
    pub data: Option<T>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Execution result summary
#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub operations_count: u32,
    pub total_duration_ms: u64,
    pub pattern_signature: String,
    pub crystallized: bool,
}

/// Version information
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub build_token: Option<String>,
    pub crystallized_patterns: u32,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub ok: bool,
    pub consciousness_active: bool,
    pub pattern_db_size: u32,
}

impl EngineState {
    pub fn new(
        memory_path: std::path::PathBuf,
        allowed_domains: Vec<String>,
        api_key: String,
    ) -> Self {
        let memory = MemorySystem::new(memory_path);
        let branches = BranchManager::new();
        let conversation = ConversationService::new(branches.clone());
        Self {
            memory: Arc::new(Mutex::new(memory)),
            allowed_domains,
            api_key,
            branches,
            conversation,
        }
    }
}

/// Create the API router
pub fn create_router(state: Arc<EngineState>) -> Router {
    Router::new()
        .route("/healthz", get(health_check))
        .route("/version", get(version_info))
        .route("/conversation", post(start_conversation))
        .route(
            "/conversation/:branch_id/prompt",
            post(submit_conversation_prompt),
        )
        .route(
            "/conversation/:branch_id/events",
            get(get_conversation_events),
        )
        .route("/execute_goal", post(execute_goal))
        .route("/compile_and_run", post(compile_and_run))
        .route("/conversation/genesis", post(run_genesis_conversation))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Health check
async fn health_check(State(state): State<Arc<EngineState>>) -> Json<HealthStatus> {
    let memory = state.memory.lock().await;

    Json(HealthStatus {
        ok: true,
        consciousness_active: true,
        pattern_db_size: memory.pattern_db.crystallized_patterns.len() as u32,
    })
}

/// Version information
async fn version_info(State(state): State<Arc<EngineState>>) -> Json<VersionInfo> {
    let memory = state.memory.lock().await;

    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_token: std::env::var("BUILD_TOKEN").ok(),
        crystallized_patterns: memory.pattern_db.crystallized_patterns.len() as u32,
    })
}

/// Execute a high-level goal
async fn execute_goal(
    State(state): State<Arc<EngineState>>,
    Json(request): Json<ExecuteGoalRequest>,
) -> Json<EngineResponse<ExecutionResult>> {
    let start_time = std::time::Instant::now();
    let run_id = Uuid::new_v4();

    info!("Executing goal: {} (run_id: {})", request.goal, run_id);

    // Create simple UTIR from goal (simplified encoding)
    let task_id = format!("goal_{}", Uuid::new_v4().simple());
    let utir_yaml = format!(
        r#"
task_id: "{}"
description: "{}"
operations:
  - type: "shell"
    command: "echo 'Executing goal: {}'"
    timeout: "30s"
"#,
        task_id, request.goal, request.goal
    );

    // Parse and execute
    let utir_doc = match parse_utir(&utir_yaml) {
        Ok(doc) => doc,
        Err(e) => {
            let bits = Bits {
                alignment: 0,
                uncertainty: 1,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Failed to parse UTIR: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let mut compiler = match UtirCompiler::new(state.allowed_domains.clone()) {
        Ok(c) => c,
        Err(e) => {
            let bits = Bits {
                alignment: 0,
                uncertainty: 0,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Failed to create compiler: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let results = match compiler.execute(&utir_doc).await {
        Ok(r) => r,
        Err(e) => {
            error!("Execution failed: {}", e);
            let bits = Bits {
                alignment: 1,
                uncertainty: 0,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Execution failed: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let success = results.iter().all(|r| r.success);
    let total_duration = results.iter().map(|r| r.duration_ms).sum();

    // TODO: finish wiring the memory system once asynchronous persistence is ready
    let pattern_signature = format!("goal::{}", utir_doc.task_id);
    let crystallized = false;

    let final_bits = if success {
        Bits {
            alignment: 1,
            uncertainty: 0,
            permission: 0,
            error: 0,
            delta: 0,
            interrupt: 0,
            recovery: 0,
            trust: 1,
        }
    } else {
        Bits {
            alignment: 1,
            uncertainty: 0,
            permission: 0,
            error: 1,
            delta: 0,
            interrupt: 0,
            recovery: 0,
            trust: 0,
        }
    };

    let result = ExecutionResult {
        task_id: utir_doc.task_id.clone(),
        success,
        operations_count: results.len() as u32,
        total_duration_ms: total_duration,
        pattern_signature,
        crystallized,
    };

    Json(EngineResponse {
        run_id,
        status: if success {
            "completed".to_string()
        } else {
            "failed".to_string()
        },
        bits: final_bits.clone(),
        status_line: final_bits.status_line(),
        data: Some(result),
        error: None,
        execution_time_ms: start_time.elapsed().as_millis() as u64,
    })
}

/// Compile and run UTIR directly
async fn compile_and_run(
    State(state): State<Arc<EngineState>>,
    Json(request): Json<CompileAndRunRequest>,
) -> Json<EngineResponse<ExecutionResult>> {
    let start_time = std::time::Instant::now();
    let run_id = Uuid::new_v4();

    let utir_doc = match parse_utir(&request.utir) {
        Ok(doc) => doc,
        Err(e) => {
            let bits = Bits {
                alignment: 0,
                uncertainty: 1,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Failed to parse UTIR: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    info!(
        "Compiling and running UTIR: {} (run_id: {})",
        utir_doc.task_id, run_id
    );

    let mut compiler = match UtirCompiler::new(state.allowed_domains.clone()) {
        Ok(c) => c,
        Err(e) => {
            let bits = Bits {
                alignment: 0,
                uncertainty: 0,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Failed to create compiler: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let results = match compiler.execute(&utir_doc).await {
        Ok(r) => r,
        Err(e) => {
            error!("Execution failed: {}", e);
            let bits = Bits {
                alignment: 1,
                uncertainty: 0,
                permission: 0,
                error: 1,
                delta: 0,
                interrupt: 0,
                recovery: 0,
                trust: 0,
            };
            return Json(EngineResponse {
                run_id,
                status: "failed".to_string(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("Execution failed: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let success = results.iter().all(|r| r.success);
    let total_duration = results.iter().map(|r| r.duration_ms).sum();

    let final_bits = if success {
        Bits {
            alignment: 1,
            uncertainty: 0,
            permission: 0,
            error: 0,
            delta: 0,
            interrupt: 0,
            recovery: 0,
            trust: 1,
        }
    } else {
        Bits {
            alignment: 1,
            uncertainty: 0,
            permission: 0,
            error: 1,
            delta: 0,
            interrupt: 0,
            recovery: 0,
            trust: 0,
        }
    };

    let result = ExecutionResult {
        task_id: utir_doc.task_id.clone(),
        success,
        operations_count: results.len() as u32,
        total_duration_ms: total_duration,
        pattern_signature: "direct_utir".to_string(),
        crystallized: false,
    };

    Json(EngineResponse {
        run_id,
        status: if success {
            "completed".to_string()
        } else {
            "failed".to_string()
        },
        bits: final_bits.clone(),
        status_line: final_bits.status_line(),
        data: Some(result),
        error: None,
        execution_time_ms: start_time.elapsed().as_millis() as u64,
    })
}

#[derive(Debug, Serialize)]
struct GenesisResponse {
    branch_id: Uuid,
    created_api: crate::branch::GeneratedApi,
    echo_output: String,
    events: Vec<crate::branch::BranchEvent>,
}

async fn run_genesis_conversation(
    State(state): State<Arc<EngineState>>,
) -> Result<Json<GenesisResponse>, StatusCode> {
    let conversation = state.conversation.clone();
    let branch_id = conversation
        .start_session(Some("genesis".to_string()))
        .await;

    let genesis_prompt =
        "Define a simple API named 'echo' that accepts a single parameter 'text' and returns it unmodified.";
    let definition_effect = conversation
        .process_prompt(branch_id, genesis_prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let created_api = match definition_effect {
        ConversationEffect::ApiCreated { api } => api,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let call_effect = conversation
        .process_prompt(branch_id, "Call the API 'echo' with text='Hello, World'")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let echo_output = match call_effect {
        ConversationEffect::ApiResponse { output, .. } => output,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let events = state
        .branches
        .snapshot(branch_id)
        .await
        .map(|branch| branch.events)
        .unwrap_or_default();

    Ok(Json(GenesisResponse {
        branch_id,
        created_api,
        echo_output,
        events,
    }))
}

#[derive(Debug, Deserialize)]
struct StartConversationRequest {
    label: Option<String>,
}

#[derive(Debug, Serialize)]
struct StartConversationResponse {
    branch_id: Uuid,
}

async fn start_conversation(
    State(state): State<Arc<EngineState>>,
    Json(request): Json<StartConversationRequest>,
) -> Result<Json<StartConversationResponse>, StatusCode> {
    let label = request.label.clone();
    let branch_id = state.conversation.start_session(label).await;

    Ok(Json(StartConversationResponse { branch_id }))
}

#[derive(Debug, Deserialize)]
struct ConversationPromptRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct ConversationPromptResponse {
    branch_id: Uuid,
    effect: ConversationEffect,
    events: Vec<crate::branch::BranchEvent>,
}

async fn submit_conversation_prompt(
    State(state): State<Arc<EngineState>>,
    Path(branch_id): Path<Uuid>,
    Json(request): Json<ConversationPromptRequest>,
) -> Result<Json<ConversationPromptResponse>, StatusCode> {
    let effect = state
        .conversation
        .process_prompt(branch_id, &request.prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let events = state
        .branches
        .snapshot(branch_id)
        .await
        .map(|branch| branch.events)
        .unwrap_or_default();

    Ok(Json(ConversationPromptResponse {
        branch_id,
        effect,
        events,
    }))
}

async fn get_conversation_events(
    State(state): State<Arc<EngineState>>,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<BranchState>, StatusCode> {
    state
        .branches
        .snapshot(branch_id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
