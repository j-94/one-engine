mod api;
mod branch;
mod compiler;
mod conversation;
mod memory;
mod meta;
mod parser;
mod utir;

use anyhow::Result;
use api::{create_router, EngineState};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::fmt::init as init_tracing;

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

    let state = Arc::new(EngineState::new(ledger_path, allowed_domains, api_key));
    let app = create_router(state);

    let listener = TcpListener::bind(bind_addr).await?;
    info!("🧠 One Engine running on http://{bind_addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
