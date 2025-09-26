use crate::branch::{BranchManager, BranchState};
use crate::chat::create_chat_router;
use crate::compiler::UtirCompiler;
use crate::conversation::{ConversationEffect, ConversationService};
use crate::memory::MemorySystem;
use crate::utir::{parse_utir, Bits, OperationResult, UtirDocument};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
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
    assert_send_sync::<AppState>();
}

#[allow(dead_code)]
fn _assert_execute_goal_future_send(app_state: Arc<AppState>) {
    fn assert_send<F: Send>(_fut: F) {}
    let fut = execute_goal(
        State(app_state),
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
    pub token_estimate_total: u32,
    pub operations: Vec<OperationReceipt>,
}

#[derive(Debug, Serialize)]
pub struct OperationReceipt {
    pub index: usize,
    pub phase: String,
    pub op: String,
    pub operation_type: String,
    pub descriptor: String,
    pub status: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output: String,
    pub output_truncated: bool,
    pub bits: Bits,
    pub metadata: HashMap<String, String>,
    pub token_estimate: u32,
}

fn build_operation_receipts(
    doc: &UtirDocument,
    results: &[OperationResult],
) -> (Vec<OperationReceipt>, u32) {
    const MAX_OUTPUT_BYTES: usize = 2048;

    let mut total_tokens = 0;
    let mut receipts = Vec::with_capacity(results.len());

    for (index, (operation, result)) in doc.operations.iter().zip(results.iter()).enumerate() {
        let descriptor = operation.descriptor();
        let mut output = result.output.clone();
        let truncated = if output.len() > MAX_OUTPUT_BYTES {
            output.truncate(MAX_OUTPUT_BYTES);
            output.push('…');
            true
        } else {
            false
        };

        let token_estimate = estimate_tokens(&descriptor, &result.output);
        total_tokens += token_estimate;

        receipts.push(OperationReceipt {
            index,
            phase: "execution".to_string(),
            op: format!("step-{}", index + 1),
            operation_type: operation.kind().to_string(),
            descriptor,
            status: if result.success {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            success: result.success,
            duration_ms: result.duration_ms,
            output,
            output_truncated: truncated,
            bits: result.bits.clone(),
            metadata: result.metadata.clone(),
            token_estimate,
        });
    }

    (receipts, total_tokens)
}

fn estimate_tokens(descriptor: &str, output: &str) -> u32 {
    let total_len = descriptor.len() + output.len();
    let total_len = u32::try_from(total_len).unwrap_or(u32::MAX);
    total_len.div_ceil(4)
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
pub fn create_router(app_state: Arc<AppState>) -> Router {
    let chat_router = create_chat_router();

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
        .merge(chat_router)
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}

/// Health check
async fn health_check(State(app_state): State<Arc<AppState>>) -> Json<HealthStatus> {
    let engine = app_state.engine();
    let memory = engine.memory.lock().await;

    Json(HealthStatus {
        ok: true,
        consciousness_active: true,
        pattern_db_size: memory.pattern_db.crystallized_patterns.len() as u32,
    })
}

/// Version information
async fn version_info(State(app_state): State<Arc<AppState>>) -> Json<VersionInfo> {
    let engine = app_state.engine();
    let memory = engine.memory.lock().await;

    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_token: std::env::var("BUILD_TOKEN").ok(),
        crystallized_patterns: memory.pattern_db.crystallized_patterns.len() as u32,
    })
}

/// Execute a high-level goal
async fn execute_goal(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ExecuteGoalRequest>,
) -> Json<EngineResponse<ExecutionResult>> {
    let start_time = std::time::Instant::now();
    let run_id = Uuid::new_v4();

    let engine_state = app_state.engine();

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

    let mut compiler = match UtirCompiler::new(engine_state.allowed_domains.clone()) {
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

    let (operations, token_estimate_total) = build_operation_receipts(&utir_doc, &results);

    let (pattern_signature, crystallized) = {
        let mut memory = engine_state.memory.lock().await;
        match memory.record_execution(&utir_doc, &results).await {
            Ok(ghost) => {
                let should_crystallize = ghost.crystallization_score > memory.ghost_threshold;
                (ghost.pattern_signature, should_crystallize)
            }
            Err(e) => {
                error!("Failed to record execution: {}", e);
                (format!("goal::{}", utir_doc.task_id), false)
            }
        }
    };

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
        token_estimate_total,
        operations,
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
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<CompileAndRunRequest>,
) -> Json<EngineResponse<ExecutionResult>> {
    let start_time = std::time::Instant::now();
    let run_id = Uuid::new_v4();

    let engine_state = app_state.engine();

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

    let mut compiler = match UtirCompiler::new(engine_state.allowed_domains.clone()) {
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

    let (operations, token_estimate_total) = build_operation_receipts(&utir_doc, &results);

    let (pattern_signature, crystallized) = {
        let mut memory = engine_state.memory.lock().await;
        match memory.record_execution(&utir_doc, &results).await {
            Ok(ghost) => {
                let should_crystallize = ghost.crystallization_score > memory.ghost_threshold;
                (ghost.pattern_signature, should_crystallize)
            }
            Err(e) => {
                error!("Failed to record execution: {}", e);
                ("direct_utir".to_string(), false)
            }
        }
    };

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
        token_estimate_total,
        operations,
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
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<GenesisResponse>, StatusCode> {
    let engine_state = app_state.engine();
    let conversation = engine_state.conversation.clone();
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

    let events = engine_state
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
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<StartConversationRequest>,
) -> Result<Json<StartConversationResponse>, StatusCode> {
    let label = request.label.clone();
    let engine_state = app_state.engine();
    let branch_id = engine_state.conversation.start_session(label).await;

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
    State(app_state): State<Arc<AppState>>,
    Path(branch_id): Path<Uuid>,
    Json(request): Json<ConversationPromptRequest>,
) -> Result<Json<ConversationPromptResponse>, StatusCode> {
    let engine_state = app_state.engine();
    let effect = engine_state
        .conversation
        .process_prompt(branch_id, &request.prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let events = engine_state
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
    State(app_state): State<Arc<AppState>>,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<BranchState>, StatusCode> {
    let engine_state = app_state.engine();
    engine_state
        .branches
        .snapshot(branch_id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
