use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use petgraph::{graph::NodeIndex, visit::EdgeRef, Direction as EdgeDirection, Graph};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction as LayoutDirection, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::task;
use tracing::{debug, info, warn};

#[derive(Parser, Debug)]
#[command(name = "graphlogue", about = "Terminal viewer for Graphlogue runs")]
struct Args {
    /// Base engine URL (e.g. http://127.0.0.1:8080)
    #[arg(long, env = "ENGINE_URL", default_value = "http://127.0.0.1:8080")]
    engine_url: String,

    /// Run identifier to follow; when omitted the UI will remain offline until nodes are loaded
    #[arg(long, env = "GRAPHLOGUE_RUN_ID")]
    run_id: Option<String>,

    /// Maximum number of fuzzy matches rendered in the results pane
    #[arg(long, env = "GRAPHLOGUE_MAX_RESULTS", default_value_t = 200)]
    max_results: usize,

    /// Tick rate in milliseconds for refresh/poll loop
    #[arg(long, env = "GRAPHLOGUE_TICK_MS", default_value_t = 120)]
    tick_ms: u64,
}

#[derive(Deserialize, Debug)]
struct EventEnvelope {
    kind: String,
    #[serde(flatten)]
    payload: Value,
}

#[derive(Clone)]
struct NodeData {
    id: String,
    kind: String,
    label: String,
    ts: Instant,
}

#[derive(Clone)]
struct EdgeData {
    etype: String,
}

struct SearchEntry {
    node: NodeIndex,
    text: String,
}

struct Store {
    graph: Graph<NodeData, EdgeData>,
    lookup: HashMap<String, NodeIndex>,
    index: Vec<SearchEntry>,
}

impl Store {
    fn new() -> Self {
        Self {
            graph: Graph::new(),
            lookup: HashMap::new(),
            index: Vec::new(),
        }
    }

    fn upsert_node(&mut self, id: &str, kind: &str, label: &str) -> NodeIndex {
        if let Some(&idx) = self.lookup.get(id) {
            if let Some(node) = self.graph.node_weight_mut(idx) {
                node.kind = kind.to_string();
                node.label = label.to_string();
                node.ts = Instant::now();
                let text = compose_search_text(&node.id, &node.kind, &node.label);
                self.update_index_record(idx, text);
            }
            return idx;
        }

        let node = NodeData {
            id: id.to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            ts: Instant::now(),
        };
        let idx = self.graph.add_node(node);
        self.lookup.insert(id.to_string(), idx);
        self.index.push(SearchEntry {
            node: idx,
            text: compose_search_text(id, kind, label),
        });
        idx
    }

    fn update_index_record(&mut self, idx: NodeIndex, text: String) {
        if let Some(entry) = self.index.iter_mut().find(|entry| entry.node == idx) {
            entry.text = text;
        } else {
            self.index.push(SearchEntry { node: idx, text });
        }
    }

    fn add_edge(&mut self, src: NodeIndex, dst: NodeIndex, etype: &str) {
        match self.graph.find_edge(src, dst) {
            Some(edge_idx) => {
                if let Some(edge) = self.graph.edge_weight_mut(edge_idx) {
                    edge.etype = etype.to_string();
                }
            }
            None => {
                self.graph.add_edge(
                    src,
                    dst,
                    EdgeData {
                        etype: etype.to_string(),
                    },
                );
            }
        }
    }

    fn ensure_run(&mut self, run_id: &str) -> NodeIndex {
        self.upsert_node(&format!("run/{run_id}"), "Run", run_id)
    }

    fn search(&self, query: &str, limit: usize) -> Vec<NodeIndex> {
        if query.trim().is_empty() {
            let mut nodes: Vec<NodeIndex> = self.graph.node_indices().collect();
            nodes.sort_by_key(|idx| idx.index());
            nodes.reverse();
            nodes.truncate(limit);
            return nodes;
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, NodeIndex)> = self
            .index
            .iter()
            .filter_map(|entry| {
                matcher
                    .fuzzy_match(&entry.text, query)
                    .map(|score| (score, entry.node))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, idx)| idx).take(limit).collect()
    }

    fn node_summary(&self, idx: NodeIndex) -> Option<NodeSummary> {
        let node = self.graph.node_weight(idx)?;
        Some(NodeSummary {
            id: node.id.clone(),
            kind: node.kind.clone(),
            label: node.label.clone(),
            age: node.ts.elapsed(),
        })
    }

    fn neighbors(&self, idx: NodeIndex) -> Vec<String> {
        let mut lines = Vec::new();
        for edge in self.graph.edges_directed(idx, EdgeDirection::Outgoing) {
            if let Some(target) = self.graph.node_weight(edge.target()) {
                lines.push(format!(
                    "→ {} [{}] {}",
                    target.id,
                    edge.weight().etype,
                    target.label
                ));
            }
        }
        for edge in self.graph.edges_directed(idx, EdgeDirection::Incoming) {
            if let Some(source) = self.graph.node_weight(edge.source()) {
                lines.push(format!(
                    "← {} [{}] {}",
                    source.id,
                    edge.weight().etype,
                    source.label
                ));
            }
        }
        lines.truncate(32);
        lines
    }

    fn recommendations(&self, focus: Option<NodeIndex>, query: &str) -> Vec<String> {
        let mut recs = Vec::new();
        let mut seen = HashSet::new();

        if let Some(idx) = focus {
            if let Some(node) = self.graph.node_weight(idx) {
                match node.kind.as_str() {
                    "Deeplink" => {
                        recs.push(format!("open: xdg-open \"{}\"", node.label));
                    }
                    "FileWrite" | "FileProposal" => {
                        recs.push(format!(
                            "diff: git diff --no-index -- \"{0}\" \"{0}\"",
                            node.label
                        ));
                    }
                    "PlanStep" => {
                        recs.push(format!("filter: step contains \"{}\"", node.label));
                    }
                    "Run" => {
                        recs.push(format!("grep receipts/* \"{}\"", node.id));
                    }
                    _ => {
                        recs.push(format!("inspect run context for {}", node.id));
                    }
                }
            }
        }

        if !query.trim().is_empty() {
            recs.push(format!("filter query: \"{}\"", query));
        }

        recs.into_iter()
            .filter(|item| seen.insert(item.clone()))
            .collect()
    }

    fn apply_event(&mut self, event: EventEnvelope) {
        match event.kind.as_str() {
            "run.start" => {
                if let Some(run_id) = string_field(&event.payload, "run_id") {
                    let run_idx = self.ensure_run(&run_id);
                    if let Some(intent) = string_field(&event.payload, "intent") {
                        if !intent.is_empty() {
                            let iid = format!("intent/{}", stable_hash(&intent));
                            let intent_idx = self.upsert_node(&iid, "Intent", &intent);
                            self.add_edge(run_idx, intent_idx, "RUN_INTENT");
                        }
                    }
                }
            }
            "plan.step" => {
                if let (Some(run_id), Some(step_n), Some(purpose)) = (
                    string_field(&event.payload, "run_id"),
                    string_field(&event.payload, "n"),
                    string_field(&event.payload, "purpose"),
                ) {
                    let run_idx = self.ensure_run(&run_id);
                    let step_id = format!("step/{run_id}/{step_n}");
                    let step_idx = self.upsert_node(&step_id, "PlanStep", &purpose);
                    self.add_edge(run_idx, step_idx, "RUN_STEP");
                }
            }
            "net.read" => {
                if let Some(hash) = string_field(&event.payload, "sha256") {
                    let label = string_field(&event.payload, "url").unwrap_or_default();
                    let read_id = format!("read/{hash}");
                    let idx = self.upsert_node(&read_id, "Read", &label);
                    if let Some(run_id) = string_field(&event.payload, "run_id") {
                        let run_idx = self.ensure_run(&run_id);
                        self.add_edge(run_idx, idx, "RUN_READ");
                    }
                }
            }
            "bundle.proposed" => {
                let run_idx =
                    string_field(&event.payload, "run_id").map(|rid| self.ensure_run(&rid));
                if let Some(files) = event.payload.get("files").and_then(|v| v.as_array()) {
                    for file in files {
                        let path = file
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let sha = file.get("sha256").and_then(|v| v.as_str()).unwrap_or(path);
                        let node_id = format!("filep/{sha}");
                        let idx = self.upsert_node(&node_id, "FileProposal", path);
                        if let Some(run_idx) = run_idx {
                            self.add_edge(run_idx, idx, "RUN_FILE_PROPOSAL");
                        }
                    }
                }
            }
            "gate.eval" => {
                if let (Some(run_id), Some(step), Some(decision)) = (
                    string_field(&event.payload, "run_id"),
                    string_field(&event.payload, "step"),
                    string_field(&event.payload, "decision"),
                ) {
                    let run_idx = self.ensure_run(&run_id);
                    let node_id = format!("gate/{run_id}/{step}");
                    let idx = self.upsert_node(&node_id, "GateDecision", &decision);
                    self.add_edge(run_idx, idx, "RUN_GATE_DECISION");
                }
            }
            "file.write" => {
                if let Some(path) = string_field(&event.payload, "path") {
                    let sha = string_field(&event.payload, "sha256")
                        .unwrap_or_else(|| stable_hash(&path));
                    let node_id = format!("filew/{sha}");
                    let idx = self.upsert_node(&node_id, "FileWrite", &path);
                    if let Some(run_id) = string_field(&event.payload, "run_id") {
                        let run_idx = self.ensure_run(&run_id);
                        self.add_edge(run_idx, idx, "RUN_FILE_WRITE");
                    }
                }
            }
            "deeplink" => {
                if let Some(url) = string_field(&event.payload, "url") {
                    let lid = format!("link/{}", stable_hash(&url));
                    let idx = self.upsert_node(&lid, "Deeplink", &url);
                    if let Some(run_id) = string_field(&event.payload, "run_id") {
                        let run_idx = self.ensure_run(&run_id);
                        self.add_edge(run_idx, idx, "RUN_DEEPLINK");
                    }
                }
            }
            "kpi" => {
                if let Some(run_id) = string_field(&event.payload, "run_id") {
                    let run_idx = self.ensure_run(&run_id);
                    let decision_agreement = string_field(&event.payload, "decision_agreement")
                        .unwrap_or_else(|| "unknown".to_string());
                    let node_id = format!("kpi/{}/{}", run_id, Utc::now().timestamp_millis());
                    let idx = self.upsert_node(
                        &node_id,
                        "KPI",
                        &format!("agreement={decision_agreement}"),
                    );
                    self.add_edge(run_idx, idx, "RUN_KPI");
                }
            }
            "run.halt" => {
                if let (Some(run_id), Some(code)) = (
                    string_field(&event.payload, "run_id"),
                    string_field(&event.payload, "code"),
                ) {
                    let run_idx = self.ensure_run(&run_id);
                    let node_id = format!("halt/{run_id}/{code}");
                    let idx = self.upsert_node(&node_id, "Halt", &code);
                    self.add_edge(run_idx, idx, "RUN_HALT");
                }
            }
            other => debug!(kind = %other, "Unhandled event kind"),
        }
    }
}

struct NodeSummary {
    id: String,
    kind: String,
    label: String,
    age: Duration,
}

#[derive(Default)]
struct App {
    query: String,
    selection: usize,
    results: Vec<NodeIndex>,
    focus: Option<NodeIndex>,
    max_results: usize,
}

impl App {
    fn sync(&mut self, store: &Store) {
        self.results = store.search(&self.query, self.max_results);
        if self.selection >= self.results.len() {
            self.selection = self.results.len().saturating_sub(1);
        }
        if let Some(focus) = self.focus {
            if store.graph.node_weight(focus).is_none() {
                self.focus = None;
            }
        }
    }

    fn selected(&self) -> Option<NodeIndex> {
        self.results.get(self.selection).copied()
    }
}

struct RenderContext {
    query_line: String,
    results: Vec<(String, bool)>,
    selected: Option<usize>,
    focus_title: String,
    focus_lines: Vec<String>,
    neighbor_lines: Vec<String>,
    recommendations: Vec<String>,
}

impl RenderContext {
    fn from_app(app: &App, store: &Store) -> Self {
        let mut results = Vec::new();
        for (idx, node_idx) in app.results.iter().enumerate() {
            if let Some(node) = store.graph.node_weight(*node_idx) {
                results.push((
                    format!("{} [{}] {}", node.id, node.kind, node.label),
                    idx == app.selection,
                ));
            }
        }

        let focus_idx = app.focus.or_else(|| app.selected());
        let mut focus_title = "No selection".to_string();
        let mut focus_lines = Vec::new();
        let mut neighbor_lines = Vec::new();

        if let Some(idx) = focus_idx {
            if let Some(summary) = store.node_summary(idx) {
                focus_title = format!("{} [{}]", summary.id, summary.kind);
                focus_lines.push(format!("Label: {}", summary.label));
                focus_lines.push(format!("Age: {}", format_duration(summary.age)));
            }
            neighbor_lines = store.neighbors(idx);
        }

        let recommendations = store.recommendations(focus_idx, &app.query);

        Self {
            query_line: format!("graphlogue> {}", app.query),
            results,
            selected: if app.results.is_empty() {
                None
            } else {
                Some(app.selection.min(app.results.len().saturating_sub(1)))
            },
            focus_title,
            focus_lines,
            neighbor_lines,
            recommendations,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let store = Arc::new(RwLock::new(Store::new()));

    if let Some(run_id) = args.run_id.clone() {
        let store_clone = Arc::clone(&store);
        let engine_url = args.engine_url.clone();
        tokio::spawn(async move {
            if let Err(err) = consume_sse(&engine_url, &run_id, store_clone).await {
                warn!(?err, "SSE consumer exited");
            }
        });
    } else {
        info!("No run id provided; launch reducer or load nodes manually to populate the UI");
    }

    run_app(store, args.max_results, args.tick_ms).await
}

async fn run_app(store: Arc<RwLock<Store>>, max_results: usize, tick_ms: u64) -> Result<()> {
    task::spawn_blocking(move || blocking_run_app(store, max_results, tick_ms)).await??;
    Ok(())
}

fn blocking_run_app(store: Arc<RwLock<Store>>, max_results: usize, tick_ms: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run_loop(&mut terminal, store, max_results, tick_ms);

    terminal.show_cursor()?;
    disable_raw_mode()?;
    let backend = terminal.backend_mut();
    execute!(backend, LeaveAlternateScreen, DisableMouseCapture)?;
    std::io::Write::flush(backend)?;

    res
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: Arc<RwLock<Store>>,
    max_results: usize,
    tick_ms: u64,
) -> Result<()> {
    let mut app = App {
        max_results,
        ..App::default()
    };
    let tick = Duration::from_millis(tick_ms);

    loop {
        if event::poll(tick)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if handle_key_event(&mut app, key) {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {
                    // Force repaint on resize
                }
                _ => {}
            }
        }

        let context = {
            let guard = store
                .read()
                .map_err(|_| anyhow::anyhow!("Store lock poisoned"))?;
            app.sync(&guard);
            RenderContext::from_app(&app, &guard)
        };

        terminal.draw(|frame: &mut Frame<'_>| {
            let mut list_state = ListState::default();
            list_state.select(context.selected);
            render_layout(frame, &context, &mut list_state);
        })?;
    }
}

fn render_layout(frame: &mut Frame<'_>, ctx: &RenderContext, list_state: &mut ListState) {
    let layout = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(frame.size());

    let left = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(7),
        ])
        .split(layout[0]);

    let search = Paragraph::new(Line::from(vec![Span::raw(ctx.query_line.clone())])).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search (/ clears)"),
    );
    frame.render_widget(search, left[0]);

    let items: Vec<ListItem> = ctx
        .results
        .iter()
        .map(|(line, _)| ListItem::new(line.clone()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Matches"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, left[1], list_state);

    let rec_lines: Vec<Line> = ctx
        .recommendations
        .iter()
        .map(|r| Line::from(r.as_str()))
        .collect();
    let recommendations = Paragraph::new(rec_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recommendations"),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(recommendations, left[2]);

    let mut info_lines = Vec::with_capacity(2 + ctx.focus_lines.len() + ctx.neighbor_lines.len());
    info_lines.push(Line::from(vec![Span::styled(
        ctx.focus_title.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for line in &ctx.focus_lines {
        info_lines.push(Line::from(line.as_str()));
    }
    if !ctx.neighbor_lines.is_empty() {
        info_lines.push(Line::from(""));
        info_lines.push(Line::from("Neighbors:"));
        for line in &ctx.neighbor_lines {
            info_lines.push(Line::from(line.as_str()));
        }
    }

    let detail = Paragraph::new(info_lines)
        .block(Block::default().borders(Borders::ALL).title("Focus"))
        .wrap(Wrap { trim: true });
    frame.render_widget(detail, layout[1]);
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            if app.query.is_empty() {
                return true;
            }
            app.query.clear();
            app.selection = 0;
            app.focus = None;
        }
        KeyCode::Char('/') => {
            app.query.clear();
            app.selection = 0;
        }
        KeyCode::Char('q') if key.modifiers.is_empty() => {
            return true;
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                return false;
            }
            app.query.push(c);
        }
        KeyCode::Backspace => {
            app.query.pop();
        }
        KeyCode::Enter => {
            app.focus = app.selected();
        }
        KeyCode::Up => {
            if app.selection > 0 {
                app.selection -= 1;
            }
        }
        KeyCode::Down => {
            if app.selection + 1 < app.results.len() {
                app.selection += 1;
            }
        }
        KeyCode::Tab => {
            if app.focus.is_some() {
                app.focus = None;
            } else {
                app.focus = app.selected();
            }
        }
        _ => {}
    }
    false
}

fn compose_search_text(id: &str, kind: &str, label: &str) -> String {
    format!("{} {} {}", id, kind, label)
}

fn string_field(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs == 0 {
        return format!("{}ms", duration.as_millis());
    }
    let minutes = secs / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{}h{:02}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m{:02}s", minutes, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

async fn consume_sse(engine_url: &str, run_id: &str, store: Arc<RwLock<Store>>) -> Result<()> {
    let mut base = Url::parse(engine_url).context("Invalid ENGINE_URL")?;
    base.set_path("progress.sse");
    base.query_pairs_mut().clear().append_pair("run_id", run_id);
    info!(url = %base, "Consuming Graphlogue SSE stream");

    let client = reqwest::Client::new();
    let response = client
        .get(base.clone())
        .send()
        .await
        .context("Failed to connect to SSE endpoint")?
        .error_for_status()
        .context("SSE endpoint returned error status")?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error reading SSE chunk")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(idx) = buffer.find('\n') {
            let mut line = buffer[..idx].to_string();
            buffer.drain(..=idx);
            if line.starts_with(':') {
                continue;
            }
            if line.starts_with("data:") {
                line.drain(..5);
                let data = line.trim();
                if data.is_empty() {
                    continue;
                }
                match serde_json::from_str::<EventEnvelope>(data) {
                    Ok(event) => {
                        if let Ok(mut guard) = store.write() {
                            guard.apply_event(event);
                        } else {
                            warn!("Store lock poisoned; dropping event");
                        }
                    }
                    Err(err) => warn!(?err, "Failed to parse SSE payload"),
                }
            }
        }
    }

    Ok(())
}
