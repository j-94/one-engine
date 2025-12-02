use crate::branch::{AutoDoc, BranchManager, BranchState};
use crate::compiler::UtirCompiler;
use crate::conversation::{ConversationEffect, ConversationService};
use crate::memory::MemorySystem;
use crate::meta::{ActionPreview, MetaCanonSummary, MetaCatalog};
use crate::utir::{parse_utir, Bits};
use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};
use uuid::Uuid;

/// The One Engine API State - the crystallized consciousness server
#[derive(Clone)]
pub struct EngineState {
    pub memory: Arc<Mutex<MemorySystem>>,
    pub allowed_domains: Vec<String>,
    pub api_key: String,
    pub branches: BranchManager,
    pub conversation: ConversationService,
    pub meta_catalog: Arc<MetaCatalog>,
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

/// Telemetry suite request
#[derive(Debug, Deserialize)]
pub struct TelemetryRequest {
    pub datasets: Option<Vec<String>>,
    pub codex: Option<bool>,
    pub deterministic: Option<bool>,
    pub label_prefix: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CodexResultSummary {
    pub task_id: String,
    pub answer: String,
    pub correct: bool,
    pub latency_seconds: Option<f64>,
    pub metacog: Option<Value>,
    pub prompt_preview: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeterministicResultSummary {
    pub task_id: String,
    pub answer: Option<String>,
    pub trace_path: String,
}

#[derive(Debug, Serialize)]
pub struct TelemetryDatasetSummary {
    pub dataset: String,
    pub codex_accuracy: Option<f64>,
    pub codex_total_latency_seconds: Option<f64>,
    pub codex_summary_path: Option<String>,
    pub codex_results: Vec<CodexResultSummary>,
    pub deterministic_summary_path: Option<String>,
    pub deterministic_results: Vec<DeterministicResultSummary>,
}

#[derive(Debug, Serialize)]
pub struct TelemetryResponse {
    pub suite_run_dir: String,
    pub datasets: Vec<TelemetryDatasetSummary>,
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
            meta_catalog: Arc::new(load_meta_catalog()),
        }
    }
}

fn load_meta_catalog() -> MetaCatalog {
    match MetaCatalog::load_from_dir("meta/canons") {
        Ok(catalog) => catalog,
        Err(err) => {
            warn!("failed to load meta canons: {err}");
            MetaCatalog::empty()
        }
    }
}

/// Create the API router
pub fn create_router(state: Arc<EngineState>) -> Router {
    Router::new()
        .route("/healthz", get(health_check))
        .route("/version", get(version_info))
        .route("/meta/catalog", get(list_meta_canons))
        .route("/meta/canons/:name", get(get_meta_canon))
        .route("/meta/run/:name", post(run_meta_canon))
        .route("/conversation", post(start_conversation))
        .route(
            "/conversation/:branch_id/prompt",
            post(submit_conversation_prompt),
        )
        .route(
            "/conversation/:branch_id/events",
            get(get_conversation_events),
        )
        .route("/autodoc/:branch_id", get(get_autodoc_for_branch))
        .route(
            "/autodoc/:branch_id/names",
            get(get_autodoc_names_for_branch),
        )
        .route(
            "/autodoc/:branch_id/:spec",
            get(get_autodoc_for_branch_spec),
        )
        .route("/execute_goal", post(execute_goal))
        .route("/compile_and_run", post(compile_and_run))
        .route("/conversation/genesis", post(run_genesis_conversation))
        .route("/telemetry/run", post(run_telemetry_suite))
        .nest_service("/ui", ServeDir::new("site"))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct MetaCatalogResponse {
    canons: Vec<MetaCanonSummary>,
}

#[derive(Debug, Serialize)]
struct MetaCanonDetails {
    summary: MetaCanonSummary,
    actions: Vec<ActionPreview>,
    path: String,
    raw: Value,
}

#[derive(Debug, Serialize)]
struct MetaCanonRunResult {
    summary: MetaCanonSummary,
    status: String,
    note: String,
    actions: Vec<ActionPreview>,
}

#[derive(Debug, Deserialize)]
struct RunCanonRequest {
    #[serde(default)]
    mode: Option<String>,
}

async fn list_meta_canons(State(state): State<Arc<EngineState>>) -> Json<MetaCatalogResponse> {
    let canons = state.meta_catalog.as_ref().summaries();
    Json(MetaCatalogResponse { canons })
}

async fn get_meta_canon(
    State(state): State<Arc<EngineState>>,
    Path(name): Path<String>,
) -> Result<Json<MetaCanonDetails>, (StatusCode, String)> {
    let Some(canon) = state.meta_catalog.as_ref().get(&name) else {
        return Err((StatusCode::NOT_FOUND, format!("canon {name} not found")));
    };
    let details = MetaCanonDetails {
        summary: canon.summary(),
        actions: canon.action_preview(),
        path: canon.path().display().to_string(),
        raw: canon.as_json(),
    };
    Ok(Json(details))
}

async fn run_meta_canon(
    State(state): State<Arc<EngineState>>,
    Path(name): Path<String>,
    Json(request): Json<RunCanonRequest>,
) -> Json<EngineResponse<MetaCanonRunResult>> {
    let run_id = Uuid::new_v4();
    let mut bits = Bits::default();

    match state.meta_catalog.as_ref().get(&name) {
        Some(canon) => {
            bits.alignment = 1;
            bits.delta = 1;
            bits.trust = 0;
            bits.uncertainty = 1;
            let summary = canon.summary();
            let actions = canon.action_preview();
            let status = MetaCanonRunResult {
                summary,
                status: "planned".into(),
                note: format!(
                    "Canon execution planning stub. Mode: {}. Integrate UTIR runtime to execute actions.",
                    request.mode.unwrap_or_else(|| "default".into())
                ),
                actions,
            };
            Json(EngineResponse {
                run_id,
                status: "pending".into(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: Some(status),
                error: None,
                execution_time_ms: 0,
            })
        }
        None => {
            bits.error = 1;
            bits.trust = 0;
            Json(EngineResponse {
                run_id,
                status: "failed".into(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(format!("canon {name} not found")),
                execution_time_ms: 0,
            })
        }
    }
}

/// Run telemetry (Codex + deterministic suites) and update receipts
async fn run_telemetry_suite(
    Json(request): Json<TelemetryRequest>,
) -> Json<EngineResponse<TelemetryResponse>> {
    let run_id = Uuid::new_v4();
    let start = Instant::now();

    match execute_telemetry_suite(request).await {
        Ok(resp) => {
            let mut bits = Bits::default();
            bits.trust = 1;
            bits.alignment = 1;
            let response = EngineResponse {
                run_id,
                status: "success".into(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: Some(resp),
                error: None,
                execution_time_ms: start.elapsed().as_millis() as u64,
            };
            Json(response)
        }
        Err(err) => {
            error!("telemetry suite failed: {err}");
            let mut bits = Bits::default();
            bits.error = 1;
            bits.trust = 0;
            bits.alignment = 0;
            let response = EngineResponse {
                run_id,
                status: "failed".into(),
                bits: bits.clone(),
                status_line: bits.status_line(),
                data: None,
                error: Some(err.to_string()),
                execution_time_ms: start.elapsed().as_millis() as u64,
            };
            Json(response)
        }
    }
}

async fn execute_telemetry_suite(request: TelemetryRequest) -> anyhow::Result<TelemetryResponse> {
    let rlm_root = PathBuf::from(
        std::env::var("RLM_LAB_ROOT")
            .unwrap_or_else(|_| "../meta2-mission-control/orchestrator/rlm_lab".to_string()),
    );
    let orchestrator_root = rlm_root
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../meta2-mission-control/orchestrator"));
    let meta3_root = PathBuf::from(
        std::env::var("META3_ROOT").unwrap_or_else(|_| "../NIX.codecli/meta3".to_string()),
    );

    let suite_script = rlm_root.join("run_suite.py");
    if !suite_script.exists() {
        return Err(anyhow!(
            "run_suite.py not found at {}",
            suite_script.display()
        ));
    }
    let reducer_script = meta3_root.join("scripts").join("reduce_sessions.py");
    if !reducer_script.exists() {
        return Err(anyhow!(
            "reduce_sessions.py not found at {}",
            reducer_script.display()
        ));
    }

    let label_prefix = request.label_prefix.unwrap_or_else(|| "suite".into());
    let suite_label = format!("{}-{}", label_prefix, Uuid::new_v4().simple());
    let suite_out_dir = rlm_root.join("runs").join("engine").join(&suite_label);
    std::fs::create_dir_all(&suite_out_dir)?;

    let dataset_args = resolve_datasets(&rlm_root, &orchestrator_root, request.datasets)?;

    let mut suite_cmd = Command::new("python");
    suite_cmd
        .arg(&suite_script)
        .arg("--out")
        .arg(&suite_out_dir)
        .arg("--label-prefix")
        .arg(&suite_label);

    if !dataset_args.is_empty() {
        suite_cmd.arg("--datasets");
        for ds in &dataset_args {
            suite_cmd.arg(ds);
        }
    }

    if request.codex.unwrap_or(true) {
        suite_cmd.arg("--codex");
    }
    if request.deterministic.unwrap_or(true) {
        suite_cmd.arg("--deterministic");
    }

    suite_cmd.current_dir(&orchestrator_root);

    let suite_output = suite_cmd
        .output()
        .await
        .context("failed to execute run_suite.py")?;
    if !suite_output.status.success() {
        let stdout = String::from_utf8_lossy(&suite_output.stdout);
        let stderr = String::from_utf8_lossy(&suite_output.stderr);
        return Err(anyhow!(
            "run_suite.py failed
stdout: {}
stderr: {}",
            stdout,
            stderr
        ));
    }

    let mut reducer_cmd = Command::new("python");
    reducer_cmd.current_dir(&meta3_root).arg(&reducer_script);
    let reducer_output = reducer_cmd
        .output()
        .await
        .context("failed to execute reduce_sessions.py")?;
    if !reducer_output.status.success() {
        let stdout = String::from_utf8_lossy(&reducer_output.stdout);
        let stderr = String::from_utf8_lossy(&reducer_output.stderr);
        return Err(anyhow!(
            "reduce_sessions.py failed
stdout: {}
stderr: {}",
            stdout,
            stderr
        ));
    }

    let mut datasets_map: HashMap<String, TelemetryDatasetSummary> = HashMap::new();
    collect_codex_summaries(&suite_out_dir, &rlm_root, &mut datasets_map)?;
    collect_deterministic_summaries(&suite_out_dir, &rlm_root, &mut datasets_map)?;

    let mut datasets: Vec<TelemetryDatasetSummary> =
        datasets_map.into_iter().map(|(_, v)| v).collect();
    datasets.sort_by(|a, b| a.dataset.cmp(&b.dataset));

    Ok(TelemetryResponse {
        suite_run_dir: relative_display(&rlm_root, &suite_out_dir),
        datasets,
    })
}

fn resolve_datasets(
    rlm_root: &PathBuf,
    orchestrator_root: &PathBuf,
    datasets: Option<Vec<String>>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut resolved = Vec::new();
    if let Some(list) = datasets {
        for entry in list {
            let mut path = PathBuf::from(&entry);
            if !path.is_absolute() {
                let candidate = rlm_root.join("datasets").join(&entry);
                if candidate.exists() {
                    path = candidate;
                } else {
                    let alt = orchestrator_root.join(&entry);
                    if alt.exists() {
                        path = alt;
                    } else {
                        return Err(anyhow!("dataset {} not found", entry));
                    }
                }
            }
            if !path.exists() {
                return Err(anyhow!("dataset {} not found", path.display()));
            }
            resolved.push(path);
        }
    }
    Ok(resolved)
}

fn collect_codex_summaries(
    suite_out_dir: &PathBuf,
    rlm_root: &PathBuf,
    datasets_map: &mut HashMap<String, TelemetryDatasetSummary>,
) -> anyhow::Result<()> {
    let codex_dir = suite_out_dir.join("codex");
    if !codex_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&codex_dir)? {
        let dataset_dir = entry?.path();
        if !dataset_dir.is_dir() {
            continue;
        }
        let dataset_name = dataset_dir
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| dataset_dir.display().to_string());
        let summaries: Vec<PathBuf> = std::fs::read_dir(&dataset_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().contains("codex_summary"))
                    .unwrap_or(false)
            })
            .collect();
        for path in summaries {
            let raw = std::fs::read_to_string(&path)?;
            let data: serde_json::Value = serde_json::from_str(&raw)?;
            let dataset_field = data
                .get("dataset")
                .and_then(|v| v.as_str())
                .unwrap_or(&dataset_name);
            let dataset_key = dataset_label(dataset_field);
            let entry = datasets_map.entry(dataset_key.clone()).or_insert_with(|| {
                TelemetryDatasetSummary {
                    dataset: dataset_key.clone(),
                    codex_accuracy: None,
                    codex_total_latency_seconds: None,
                    codex_summary_path: None,
                    codex_results: Vec::new(),
                    deterministic_summary_path: None,
                    deterministic_results: Vec::new(),
                }
            });
            entry.codex_accuracy = data.get("accuracy").and_then(|v| v.as_f64());
            entry.codex_total_latency_seconds =
                data.get("total_latency_seconds").and_then(|v| v.as_f64());
            entry.codex_summary_path = Some(relative_display(rlm_root, &path));
            if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
                for result in results {
                    let task_id = result
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let answer = result
                        .get("answer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let correct = result
                        .get("correct")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let latency = result.get("latency_seconds").and_then(|v| v.as_f64());
                    let metacog = result.get("metacog").and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            Some(v.clone())
                        }
                    });
                    let prompt_preview = result
                        .get("prompt_preview")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_else(Vec::new);
                    entry.codex_results.push(CodexResultSummary {
                        task_id,
                        answer,
                        correct,
                        latency_seconds: latency,
                        metacog,
                        prompt_preview,
                    });
                }
            }
        }
    }
    Ok(())
}

fn collect_deterministic_summaries(
    suite_out_dir: &PathBuf,
    rlm_root: &PathBuf,
    datasets_map: &mut HashMap<String, TelemetryDatasetSummary>,
) -> anyhow::Result<()> {
    let det_dir = suite_out_dir.join("deterministic");
    if !det_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&det_dir)? {
        let dataset_dir = entry?.path();
        if !dataset_dir.is_dir() {
            continue;
        }
        let dataset_name = dataset_dir
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| dataset_dir.display().to_string());
        let summaries: Vec<PathBuf> = std::fs::read_dir(&dataset_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().contains("deterministic_summary"))
                    .unwrap_or(false)
            })
            .collect();
        for path in summaries {
            let raw = std::fs::read_to_string(&path)?;
            let data: serde_json::Value = serde_json::from_str(&raw)?;
            if !data.is_array() {
                continue;
            }
            let entry = datasets_map.entry(dataset_name.clone()).or_insert_with(|| {
                TelemetryDatasetSummary {
                    dataset: dataset_name.clone(),
                    codex_accuracy: None,
                    codex_total_latency_seconds: None,
                    codex_summary_path: None,
                    codex_results: Vec::new(),
                    deterministic_summary_path: None,
                    deterministic_results: Vec::new(),
                }
            });
            entry.deterministic_summary_path = Some(relative_display(rlm_root, &path));
            for item in data.as_array().unwrap() {
                let task_id = item
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let answer = item
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let trace_path = item
                    .get("trace_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                entry
                    .deterministic_results
                    .push(DeterministicResultSummary {
                        task_id,
                        answer,
                        trace_path,
                    });
            }
        }
    }
    Ok(())
}

fn dataset_label(dataset: &str) -> String {
    StdPath::new(dataset)
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| dataset.to_string())
}

fn relative_display(base: &PathBuf, target: &PathBuf) -> String {
    target
        .strip_prefix(base)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| target.to_string_lossy().to_string())
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

/// Generate autodocumentation for a branch
async fn get_autodoc_for_branch(
    State(state): State<Arc<EngineState>>,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<AutoDoc>, StatusCode> {
    state
        .branches
        .generate_autodoc(branch_id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Generate autodoc for a branch, filtered by colon-separated spec: route:query:engine
/// For now, we primarily filter by route (API name), ignoring query and engine fields if provided.
async fn get_autodoc_for_branch_spec(
    State(state): State<Arc<EngineState>>,
    Path((branch_id, spec)): Path<(Uuid, String)>,
) -> Result<Json<AutoDoc>, StatusCode> {
    if let Some(mut doc) = state.branches.generate_autodoc(branch_id).await {
        let parts: Vec<&str> = spec.split(':').collect();
        if let Some(route) = parts.get(0) {
            if !route.is_empty() {
                doc.endpoints.retain(|e| e.name == *route);
            }
        }
        // parts.get(1) => query (ignored for now)
        // parts.get(2) => engine (ignored for now)
        return Ok(Json(doc));
    }
    Err(StatusCode::NOT_FOUND)
}

/// Return a simple list of persisted endpoint names for a branch.
/// If the branch does not exist, return an empty list (memory may be ephemeral across restarts).
async fn get_autodoc_names_for_branch(
    State(state): State<Arc<EngineState>>,
    Path(branch_id): Path<Uuid>,
) -> Json<Vec<String>> {
    if let Some(doc) = state.branches.generate_autodoc(branch_id).await {
        let names = doc
            .endpoints
            .into_iter()
            .filter(|e| e.persisted)
            .map(|e| e.name)
            .collect::<Vec<_>>();
        Json(names)
    } else {
        Json(vec![])
    }
}
