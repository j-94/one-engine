use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Clone)]
struct AutoDoc { endpoints: Vec<EndpointDoc> }
#[derive(Debug, Deserialize, Clone)]
struct EndpointDoc { name: String, description: String, parameters: Vec<String>, persisted: bool, examples: Vec<String> }

#[derive(Debug, Deserialize, Clone)]
struct StartConversationResponse { branch_id: String }

struct App {
    base_url: String,
    branch_id: String,
    input: String,
    messages: Vec<String>,
    endpoints: Vec<String>,
    status: String,
    last_refresh: Instant,
    client: Client,
}

impl App {
    fn new(base_url: String, branch_id: String) -> Self {
        Self {
            base_url,
            branch_id,
            input: String::new(),
            messages: Vec::new(),
            endpoints: Vec::new(),
            status: "Ready".to_string(),
            last_refresh: Instant::now() - Duration::from_secs(3600),
            client: Client::new(),
        }
    }

    fn ensure_branch(&mut self) {
        if self.branch_id.trim().is_empty() {
            let url = format!("{}/conversation", self.base_url);
            if let Ok(resp) = self.client.post(&url).json(&serde_json::json!({"label": "tui"})).send() {
                if let Ok(data) = resp.json::<StartConversationResponse>() {
                    self.branch_id = data.branch_id;
                    self.status = format!("New branch: {}", self.branch_id);
                }
            }
        }
    }

    fn send_prompt(&mut self, prompt: &str) {
        self.ensure_branch();
        let url = format!("{}/conversation/{}/prompt", self.base_url, self.branch_id);
        match self.client.post(&url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&serde_json::json!({"prompt": prompt})).unwrap())
            .send() {
            Ok(resp) => {
                self.messages.push(format!("> {}", prompt));
                if let Ok(v) = resp.json::<serde_json::Value>() {
                    let effect = v.get("effect").cloned().unwrap_or(serde_json::json!({"Unknown":{}}));
                    self.messages.push(format!("{:?}", effect));
                } else {
                    self.messages.push("(non-JSON response)".to_string());
                }
            }
            Err(e) => self.status = format!("send error: {}", e),
        }
    }

    fn refresh_autodoc(&mut self) {
        self.ensure_branch();
        let url = format!("{}/autodoc/{}", self.base_url, self.branch_id);
        match self.client.get(&url).send() {
            Ok(resp) => {
                if let Ok(doc) = resp.json::<AutoDoc>() {
                    self.endpoints = doc.endpoints.into_iter().map(|e| e.name).collect();
                    self.last_refresh = Instant::now();
                }
            }
            Err(e) => self.status = format!("autodoc error: {}", e),
        }
    }
}

fn main() -> io::Result<()> {
    let base_url = std::env::var("ENGINE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:7777".to_string());
    let branch_id = std::env::var("ENGINE_BRANCH_ID").unwrap_or_else(|_| {
        std::fs::read_to_string("out_one_engine/branch_id.txt").unwrap_or_default()
    }).trim().to_string();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(base_url, branch_id);
    app.refresh_autodoc();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(size);

            let top = Paragraph::new(Line::from(vec![
                Span::raw("TUI • "),
                Span::styled(&app.base_url, Style::default().fg(Color::Cyan)),
                Span::raw(" • branch: "),
                Span::styled(&app.branch_id, Style::default().fg(Color::Yellow)),
                Span::raw(" • status: "),
                Span::raw(&app.status),
                Span::raw("  (Enter=send, Ctrl-R=refresh, Ctrl-N=new branch, Ctrl-G=graph UI, Ctrl-C=quit)"),
            ]))
            .block(Block::default());
            f.render_widget(top, layout[0]);

            let mid = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);

            let items: Vec<ListItem> = app.messages.iter().rev().take(200).rev().map(|m| ListItem::new(m.clone())).collect();
            let chat = List::new(items).block(Block::default().borders(Borders::ALL).title("Chat"));
            f.render_widget(chat, mid[0]);

            let ep_items: Vec<ListItem> = app.endpoints.iter().map(|e| ListItem::new(e.clone())).collect();
            let ep_list = List::new(ep_items).block(Block::default().borders(Borders::ALL).title("Endpoints"));
            f.render_widget(ep_list, mid[1]);

            let input = Paragraph::new(app.input.as_str())
                .block(Block::default().borders(Borders::ALL).title("Prompt"));
            f.render_widget(input, layout[2]);
            // Put cursor at end of input
            f.set_cursor(layout[2].x + 1 + app.input.len() as u16, layout[2].y + 1);
        })?;

        // Auto-refresh autodoc every 5s
        if app.last_refresh.elapsed() > Duration::from_secs(5) {
            app.refresh_autodoc();
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Esc) => break,
                        (KeyModifiers::CONTROL, KeyCode::Char('r')) => app.refresh_autodoc(),
                        (KeyModifiers::CONTROL, KeyCode::Char('n')) => { app.branch_id.clear(); app.ensure_branch(); app.refresh_autodoc(); },
                        (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(format!("ENGINE_BASE_URL={} ENGINE_BRANCH_ID={} target/debug/graph_ui >/tmp/one_engine_graph_ui.log 2>&1 &", app.base_url, app.branch_id))
                                .spawn();
                            app.status = "Graph UI launched".to_string();
                        }
                        (_, KeyCode::Enter) => {
                            let prompt = std::mem::take(&mut app.input);
                            if !prompt.trim().is_empty() { app.send_prompt(&prompt); }
                        }
                        (_, KeyCode::Backspace) => { app.input.pop(); }
                        (_, KeyCode::Char(c)) => app.input.push(c),
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}