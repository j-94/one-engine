mod api;
mod branch;
mod chat;
mod compiler;
mod conversation;
mod events;
mod memory;
mod parser;
mod schema;
mod utir;

use anyhow::Result;
use api::{create_router, EngineState};
use chat::GenerativeChatEngine;
use schema::{default_schema, SchemaEvolutionEngine};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::fmt::init as init_tracing;

#[derive(Clone)]
pub struct AppState {
    engine: Arc<EngineState>,
    chat: Arc<GenerativeChatEngine>,
}

impl AppState {
    pub fn new(engine: Arc<EngineState>, chat: Arc<GenerativeChatEngine>) -> Self {
        Self { engine, chat }
    }

    pub fn engine(&self) -> Arc<EngineState> {
        Arc::clone(&self.engine)
    }

    pub fn chat(&self) -> Arc<GenerativeChatEngine> {
        Arc::clone(&self.chat)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let bind_addr: SocketAddr = std::env::var("ENGINE_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7777".to_string())
        .parse()
        .expect("ENGINE_BIND_ADDR must be a valid socket address");

    let memory_path =
        std::env::var("ENGINE_MEMORY_PATH").unwrap_or_else(|_| "memory/ledger.jsonl".to_string());

    let allowed_domains = std::env::var("ENGINE_ALLOWED_DOMAINS")
        .unwrap_or_else(|_| "localhost,127.0.0.1".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let api_key = std::env::var("ENGINE_API_KEY").unwrap_or_else(|_| "change-me".to_string());

    let ledger_path = PathBuf::from(&memory_path);
    if let Some(parent) = ledger_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let engine_state = Arc::new(EngineState::new(ledger_path, allowed_domains, api_key));

    let base_schema = default_schema();
    let schema_engine = SchemaEvolutionEngine::new(base_schema);
    let chat_engine = Arc::new(GenerativeChatEngine::new(schema_engine));

    let app_state = Arc::new(AppState::new(engine_state, chat_engine));
    let app = create_router(Arc::clone(&app_state));

    let listener = TcpListener::bind(bind_addr).await?;
    info!("🧠 One Engine running on http://{bind_addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
