// Minimal egui/eframe app to visualize One Engine branch autodoc as a graph with fzf-like search.
// Run with: cargo run --bin graph_ui

use eframe::{egui, App};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Undirected;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::f32;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
struct AutoDoc {
    branch_id: String,
    label: Option<String>,
    endpoints: Vec<EndpointDoc>,
}

#[derive(Debug, Deserialize, Clone)]
struct EndpointDoc {
    name: String,
    description: String,
    parameters: Vec<String>,
    persisted: bool,
    examples: Vec<String>,
}

// Catalog for patterns, features, achievements
#[derive(Debug, Deserialize, Clone)]
struct Catalog {
    patterns: Vec<CatalogPattern>,
    features: Vec<CatalogFeature>,
    achievements: Vec<CatalogAchievement>,
}
#[derive(Debug, Deserialize, Clone)]
struct CatalogPattern {
    id: String,
    title: String,
    summary: String,
    tags: Vec<String>,
    docs_path: String,
    pinned: bool,
    level: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
struct CatalogFeature {
    id: String,
    title: String,
    summary: String,
    docs_path: String,
    pinned: bool,
}
#[derive(Debug, Deserialize, Clone)]
struct CatalogAchievement {
    id: String,
    title: String,
    description: String,
    target_id: String, // pattern or feature id
    pinned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NodeKind {
    Endpoint,
    Pattern,
    Feature,
}

// Minimal template registry for one-click prompts in the Graph UI
struct TemplateParam {
    key: &'static str,
}

struct TemplateSpec {
    name: &'static str,
    description: &'static str,
    params: &'static [TemplateParam],
    // Closure to build the define prompt
    define_prompt: fn(&TemplateSpec) -> String,
}

fn define_prompt_uppercase(_: &TemplateSpec) -> String {
    "Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase."
        .to_string()
}
fn define_prompt_reverse(_: &TemplateSpec) -> String {
    "Define a persistent API named 'reverse' that accepts 'text' and returns the reversed string."
        .to_string()
}
fn define_prompt_slugify(_: &TemplateSpec) -> String {
    "Define a persistent API named 'slugify' that accepts 'text' and returns a URL-safe slug."
        .to_string()
}
fn define_prompt_counter(_: &TemplateSpec) -> String {
    "Define a persistent API named 'counter' that increments an internal counter and returns it each call.".to_string()
}
fn define_prompt_replace(_: &TemplateSpec) -> String {
    "Define a persistent API named 'replace' that accepts 'text', 'from', and 'to' and returns the text with occurrences of 'from' replaced by 'to'.".to_string()
}
fn define_prompt_concat(_: &TemplateSpec) -> String {
    "Define a persistent API named 'concat' that accepts 'a' and 'b' and returns their concatenation.".to_string()
}

static TEMPLATE_SPECS: &[TemplateSpec] = &[
    TemplateSpec {
        name: "uppercase",
        description: "text -> TEXT",
        params: &[TemplateParam { key: "text" }],
        define_prompt: define_prompt_uppercase,
    },
    TemplateSpec {
        name: "reverse",
        description: "text -> reversed",
        params: &[TemplateParam { key: "text" }],
        define_prompt: define_prompt_reverse,
    },
    TemplateSpec {
        name: "slugify",
        description: "text -> url-safe slug",
        params: &[TemplateParam { key: "text" }],
        define_prompt: define_prompt_slugify,
    },
    TemplateSpec {
        name: "counter",
        description: "() -> incrementing count",
        params: &[],
        define_prompt: define_prompt_counter,
    },
    TemplateSpec {
        name: "replace",
        description: "text, from, to -> replaced",
        params: &[
            TemplateParam { key: "text" },
            TemplateParam { key: "from" },
            TemplateParam { key: "to" },
        ],
        define_prompt: define_prompt_replace,
    },
    TemplateSpec {
        name: "concat",
        description: "a, b -> concatenated",
        params: &[TemplateParam { key: "a" }, TemplateParam { key: "b" }],
        define_prompt: define_prompt_concat,
    },
];

struct GraphUiApp {
    base_url: String,
    branch_id: String,
    autodoc: Option<AutoDoc>,
    catalog: Option<Catalog>,
    events: Option<JsonValue>,
    // current total events (N) and selected cutoff (K)
    timeline_total: usize,
    timeline_cutoff: usize,
    graph: Graph<String, String, Undirected>,
    node_positions: HashMap<NodeIndex, egui::Pos2>,
    velocities: HashMap<NodeIndex, egui::Vec2>,
    node_kinds: HashMap<NodeIndex, NodeKind>,
    by_name_index: HashMap<String, NodeIndex>,
    search: String,
    show_persisted_only: bool,
    show_endpoints: bool,
    show_patterns: bool,
    show_features: bool,
    relax_layout: bool,
    selected: Option<NodeIndex>,
    dragging: bool,
    // Templates UI state
    show_templates: bool,
    template_search: String,
    template_inputs: HashMap<String, HashMap<String, String>>, // template_name -> (param->value)
    // Onboard context UI
    context_input: String,
    client: Client,
    status: String,
}

impl GraphUiApp {
    fn new(base_url: String, branch_id: String) -> Self {
        Self {
            base_url,
            branch_id,
            autodoc: None,
            catalog: None,
            events: None,
            timeline_total: 0,
            timeline_cutoff: 0,
            graph: Graph::default(),
            node_positions: HashMap::new(),
            velocities: HashMap::new(),
            node_kinds: HashMap::new(),
            by_name_index: HashMap::new(),
            search: String::new(),
            show_persisted_only: false,
            show_endpoints: true,
            show_patterns: true,
            show_features: true,
            relax_layout: true,
            selected: None,
            dragging: false,
            show_templates: false,
            template_search: String::new(),
            template_inputs: HashMap::new(),
            context_input: String::new(),
            client: Client::new(),
            status: "Ready".to_string(),
        }
    }

    fn feedback_counts(&self) -> Option<(usize, usize, usize)> {
        let events = self.events.as_ref()?;
        let arr = events.get("events")?.as_array()?;
        let k = self.timeline_cutoff.min(arr.len());
        let mut gen_ct = 0usize;
        let mut appr_ct = 0usize;
        let mut call_ct = 0usize;
        for ev in arr.iter().take(k) {
            if ev.get("ApiGenerated").is_some() {
                gen_ct += 1;
            }
            if ev.get("ApiCalled").is_some() {
                call_ct += 1;
            }
            if let Some(pi) = ev
                .get("ParsedIntent")
                .and_then(|x| x.get("description"))
                .and_then(|x| x.as_str())
            {
                if pi.contains("ApprovePattern") || pi.contains("approval:") {
                    appr_ct += 1;
                }
            }
        }
        Some((gen_ct, appr_ct, call_ct))
    }

    fn fetch_autodoc(&mut self) {
        let url = format!("{}/autodoc/{}", self.base_url, self.branch_id);
        match self.client.get(&url).send() {
            Ok(resp) => match resp.json::<AutoDoc>() {
                Ok(doc) => {
                    self.autodoc = Some(doc);
                    self.rebuild_graph();
                    self.status = "Autodoc loaded".to_string();
                }
                Err(e) => self.status = format!("Failed to parse autodoc: {}", e),
            },
            Err(e) => self.status = format!("Failed to GET {}: {}", url, e),
        }
    }

    fn fetch_catalog(&mut self) {
        // Load from local file path docs/catalog.json if present
        let path = std::path::Path::new("docs/catalog.json");
        match fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str::<Catalog>(&s) {
                Ok(cat) => {
                    self.catalog = Some(cat);
                    self.rebuild_graph();
                    self.status = "Catalog loaded".to_string();
                }
                Err(e) => self.status = format!("Failed to parse catalog: {}", e),
            },
            Err(_) => {
                // catalog optional; ignore if missing
            }
        }
    }

    fn fetch_events(&mut self) {
        let url = format!("{}/conversation/{}/events", self.base_url, self.branch_id);
        match self.client.get(&url).send() {
            Ok(resp) => match resp.json::<JsonValue>() {
                Ok(val) => {
                    // update events and timeline bounds
                    self.timeline_total = val
                        .get("events")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    // default cutoff to total if not yet set or larger than total
                    if self.timeline_cutoff == 0 || self.timeline_cutoff > self.timeline_total {
                        self.timeline_cutoff = self.timeline_total;
                    }
                    self.events = Some(val);
                    self.rebuild_edges_from_events();
                }
                Err(e) => self.status = format!("Failed to parse events: {}", e),
            },
            Err(e) => self.status = format!("Failed to GET {}: {}", url, e),
        }
    }

    fn rebuild_graph(&mut self) {
        self.graph = Graph::default();
        self.node_positions.clear();
        self.velocities.clear();
        self.by_name_index.clear();
        self.node_kinds.clear();

        // Build nodes for endpoints
        if self.show_endpoints {
            if let Some(doc) = &self.autodoc {
                let mut nodes: Vec<(String, NodeIndex)> = Vec::new();
                for ep in &doc.endpoints {
                    let idx = self.graph.add_node(ep.name.clone());
                    self.node_kinds.insert(idx, NodeKind::Endpoint);
                    self.by_name_index.insert(ep.name.clone(), idx);
                    nodes.push((ep.name.clone(), idx));
                }
                // Layout ring 1
                let n = nodes.len().max(1) as f32;
                let radius = 220.0;
                let center = egui::pos2(0.0, 0.0);
                for (i, (_name, idx)) in nodes.iter().enumerate() {
                    let t = i as f32 / n * std::f32::consts::TAU;
                    let pos = egui::pos2(center.x + radius * t.cos(), center.y + radius * t.sin());
                    self.node_positions.insert(*idx, pos);
                    self.velocities.insert(*idx, egui::vec2(0.0, 0.0));
                }
            }
        }

        // Build nodes for patterns and features from catalog
        if let Some(cat) = &self.catalog {
            if self.show_patterns {
                let mut nodes: Vec<(String, NodeIndex)> = Vec::new();
                for p in &cat.patterns {
                    let label = format!("P: {}", p.title);
                    let idx = self.graph.add_node(label.clone());
                    self.node_kinds.insert(idx, NodeKind::Pattern);
                    self.by_name_index.insert(label.clone(), idx);
                    nodes.push((label, idx));
                }
                // Layout ring 2
                let n = nodes.len().max(1) as f32;
                let radius = 340.0;
                let center = egui::pos2(0.0, 0.0);
                for (i, (_name, idx)) in nodes.iter().enumerate() {
                    let t = i as f32 / n * std::f32::consts::TAU + 0.4;
                    let pos = egui::pos2(center.x + radius * t.cos(), center.y + radius * t.sin());
                    self.node_positions.insert(*idx, pos);
                    self.velocities.insert(*idx, egui::vec2(0.0, 0.0));
                }
            }
            if self.show_features {
                let mut nodes: Vec<(String, NodeIndex)> = Vec::new();
                for f in &cat.features {
                    let label = format!("F: {}", f.title);
                    let idx = self.graph.add_node(label.clone());
                    self.node_kinds.insert(idx, NodeKind::Feature);
                    self.by_name_index.insert(label.clone(), idx);
                    nodes.push((label, idx));
                }
                // Layout ring 3
                let n = nodes.len().max(1) as f32;
                let radius = 460.0;
                let center = egui::pos2(0.0, 0.0);
                for (i, (_name, idx)) in nodes.iter().enumerate() {
                    let t = i as f32 / n * std::f32::consts::TAU + 0.8;
                    let pos = egui::pos2(center.x + radius * t.cos(), center.y + radius * t.sin());
                    self.node_positions.insert(*idx, pos);
                    self.velocities.insert(*idx, egui::vec2(0.0, 0.0));
                }
            }
        }

        // Edges derived after events fetched
        self.rebuild_edges_from_events();
    }

    fn apply_force_layout_step(&mut self) {
        if self.node_positions.is_empty() {
            return;
        }
        // Basic Fruchterman-Reingold-like iteration
        let area = 800.0 * 800.0;
        let k = (area / (self.node_positions.len() as f32 + 1.0)).sqrt();
        let temperature = 0.9; // small step
        let mut disp: HashMap<NodeIndex, egui::Vec2> = HashMap::new();
        for &ni in self.node_positions.keys() {
            disp.insert(ni, egui::vec2(0.0, 0.0));
        }
        // Repulsive forces
        let keys: Vec<NodeIndex> = self.node_positions.keys().copied().collect();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                let ni = keys[i];
                let nj = keys[j];
                let pi = *self.node_positions.get(&ni).unwrap();
                let pj = *self.node_positions.get(&nj).unwrap();
                let delta = pi - pj;
                let dist = (delta.x.powi(2) + delta.y.powi(2)).sqrt().max(0.01);
                let force = (k * k) / dist;
                let dir = egui::vec2(delta.x / dist, delta.y / dist);
                *disp.get_mut(&ni).unwrap() += dir * force;
                *disp.get_mut(&nj).unwrap() -= dir * force;
            }
        }
        // Attractive forces along edges (stronger for data-flow)
        for e in self.graph.edge_references() {
            let a = e.source();
            let b = e.target();
            if let (Some(pa), Some(pb)) = (self.node_positions.get(&a), self.node_positions.get(&b))
            {
                let delta = *pa - *pb;
                let dist = (delta.x.powi(2) + delta.y.powi(2)).sqrt().max(0.01);
                let mut force = (dist * dist) / k;
                if e.weight() == "data" {
                    force *= 1.5;
                }
                let dir = egui::vec2(delta.x / dist, delta.y / dist);
                *disp.get_mut(&a).unwrap() -= dir * force;
                *disp.get_mut(&b).unwrap() += dir * force;
            }
        }
        // Update positions with capped displacement
        for (&ni, d) in disp.iter() {
            let v = self.velocities.get_mut(&ni).unwrap();
            *v = (*v + *d) * 0.5; // damp
            let p = self.node_positions.get_mut(&ni).unwrap();
            let len = (v.x.powi(2) + v.y.powi(2)).sqrt();
            let step = if len > temperature {
                egui::vec2(v.x / len * temperature, v.y / len * temperature)
            } else {
                *v
            };
            *p += step;
        }
    }

    fn rebuild_edges_from_events(&mut self) {
        if self.autodoc.is_none() || self.events.is_none() {
            return;
        }
        let events = self.events.as_ref().unwrap();
        let mut calls: Vec<String> = Vec::new();
        if let Some(arr) = events.get("events").and_then(|v| v.as_array()) {
            let k = self.timeline_cutoff.min(arr.len());
            for ev in arr.iter().take(k) {
                if let Some(name) = ev
                    .get("ApiCalled")
                    .and_then(|n| n.get("name"))
                    .and_then(|s| s.as_str())
                {
                    calls.push(name.to_string());
                }
            }
        }
        // Add edges for consecutive calls (unique pairs)
        let mut seen: HashSet<(NodeIndex, NodeIndex, &'static str)> = HashSet::new();
        for w in calls.windows(2) {
            if let [a, b] = &w {
                if a != b {
                    if let (Some(&ia), Some(&ib)) =
                        (self.by_name_index.get(a), self.by_name_index.get(b))
                    {
                        let (x, y) = if ia.index() <= ib.index() {
                            (ia, ib)
                        } else {
                            (ib, ia)
                        };
                        if seen.insert((x, y, "seq")) {
                            self.graph.add_edge(x, y, String::from("seq"));
                        }
                    }
                }
            }
        }
        // Add edges for DataFlow events (bounded by K)
        if let Some(arr) = events.get("events").and_then(|v| v.as_array()) {
            let k = self.timeline_cutoff.min(arr.len());
            for ev in arr.iter().take(k) {
                if let Some(df) = ev.get("DataFlow") {
                    if let (Some(from), Some(to)) = (
                        df.get("from").and_then(|s| s.as_str()),
                        df.get("to").and_then(|s| s.as_str()),
                    ) {
                        if let (Some(&ia), Some(&ib)) =
                            (self.by_name_index.get(from), self.by_name_index.get(to))
                        {
                            let (x, y) = if ia.index() <= ib.index() {
                                (ia, ib)
                            } else {
                                (ib, ia)
                            };
                            if seen.insert((x, y, "data")) {
                                self.graph.add_edge(x, y, String::from("data"));
                            }
                        }
                    }
                }
            }
        }
    }

    fn filtered_indices(&self) -> Vec<(NodeIndex, i64)> {
        // Fuzzy match on name + description + examples, with persisted-only filter
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(NodeIndex, i64)> = Vec::new();
        let Some(doc) = &self.autodoc else {
            return scored;
        };
        // Map names to EndpointDoc for context
        let mut by_name: HashMap<&str, &EndpointDoc> = HashMap::new();
        for ep in &doc.endpoints {
            by_name.insert(ep.name.as_str(), ep);
        }
        let search_empty = self.search.trim().is_empty();
        for ni in self.graph.node_indices() {
            if let Some(name) = self.graph.node_weight(ni) {
                let ep = by_name.get(name.as_str());
                if self.show_persisted_only {
                    if let Some(ep) = ep {
                        if !ep.persisted {
                            continue;
                        }
                    }
                }
                if search_empty {
                    scored.push((ni, 0));
                    continue;
                }
                let mut hay = name.clone();
                if let Some(ep) = ep {
                    hay.push_str(" ");
                    hay.push_str(&ep.description);
                    for ex in &ep.examples {
                        hay.push_str(" ");
                        hay.push_str(ex);
                    }
                }
                if let Some(score) = matcher.fuzzy_match(hay.as_str(), &self.search) {
                    scored.push((ni, score));
                }
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored
    }
}

fn slugify(input: &str) -> String {
    let mut s = input.to_lowercase();
    s = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

impl App for GraphUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Base URL:");
                ui.text_edit_singleline(&mut self.base_url);
                ui.label("Branch:");
                ui.text_edit_singleline(&mut self.branch_id);
                if ui.button("Load").clicked() {
                    self.fetch_autodoc();
                    self.fetch_catalog();
                    self.fetch_events();
                }
                if ui.button("Reload all").clicked() {
                    self.fetch_autodoc();
                    self.fetch_catalog();
                    self.fetch_events();
                }
                ui.toggle_value(&mut self.relax_layout, "Relax layout");
                ui.label(format!("Status: {}", self.status));
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Search:");
                let _ = ui.text_edit_singleline(&mut self.search);
                ui.toggle_value(&mut self.show_persisted_only, "Persisted only");
                ui.toggle_value(&mut self.show_templates, "Templates");
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.show_endpoints, "Endpoints");
                ui.toggle_value(&mut self.show_patterns, "Patterns");
                ui.toggle_value(&mut self.show_features, "Features");
                if ui.button("Apply layers").clicked() {
                    self.rebuild_graph();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Events:");
                if self.timeline_total == 0 {
                    ui.add_enabled(
                        false,
                        egui::Slider::new(&mut self.timeline_cutoff, 0..=0).text("0 / 0"),
                    );
                } else {
                    // ensure cutoff within bounds
                    if self.timeline_cutoff > self.timeline_total {
                        self.timeline_cutoff = self.timeline_total;
                    }
                    let label = format!("{} / {}", self.timeline_cutoff, self.timeline_total);
                    ui.add(
                        egui::Slider::new(&mut self.timeline_cutoff, 0..=self.timeline_total)
                            .text(label),
                    );
                    if ui.button("Reset").clicked() {
                        self.timeline_cutoff = self.timeline_total;
                    }
                }
                ui.label("(Move slider, then click Load/Reload)");
            });
            // Feedback summary
            if let Some((gen_ct, appr_ct, call_ct)) = self.feedback_counts() {
                ui.label(format!(
                    "Feedback: generated={} approvals={} calls={}",
                    gen_ct, appr_ct, call_ct
                ));
            }
        });

        egui::SidePanel::left("left")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Endpoints");
                let indices = self.filtered_indices();
                for (ni, _score) in indices.iter().take(100) {
                    if let Some(name) = self.graph.node_weight(*ni) {
                        let clicked = ui
                            .selectable_label(self.selected == Some(*ni), name)
                            .clicked();
                        if clicked {
                            self.selected = Some(*ni);
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Graph view");
            ui.separator();
            // Draw graph in a large interactable area
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let to_screen = egui::emath::RectTransform::from_to(
                egui::Rect::from_center_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 700.0)),
                response.rect,
            );

            let filtered: HashSet<NodeIndex> = self
                .filtered_indices()
                .into_iter()
                .map(|(ni, _)| ni)
                .collect();

            // Draw edges (sequence co-occurrence = gray, data-flow = orange)
            for edge in self.graph.edge_references() {
                let (a, b) = (edge.source(), edge.target());
                if let (Some(pa), Some(pb)) =
                    (self.node_positions.get(&a), self.node_positions.get(&b))
                {
                    let sa = to_screen.transform_pos(*pa);
                    let sb = to_screen.transform_pos(*pb);
                    let w = edge.weight();
                    let col = if w == "data" {
                        egui::Color32::from_rgb(255, 140, 0)
                    } else {
                        egui::Color32::from_gray(160)
                    };
                    painter.line_segment(
                        [sa, sb],
                        egui::Stroke {
                            width: 2.0,
                            color: col,
                        },
                    );
                }
            }

            // Layout step
            if self.relax_layout {
                self.apply_force_layout_step();
            }

            // Draw nodes
            for ni in self.graph.node_indices() {
                if let Some(pos) = self.node_positions.get(&ni) {
                    let screen_pos = to_screen.transform_pos(*pos);
                    let active = filtered.is_empty() || filtered.contains(&ni);
                    let mut color = if active {
                        egui::Color32::from_rgb(0, 128, 255)
                    } else {
                        egui::Color32::from_gray(120)
                    };
                    // Node kind coloring
                    if let Some(kind) = self.node_kinds.get(&ni) {
                        match kind {
                            NodeKind::Endpoint => {
                                if let Some(doc) = &self.autodoc {
                                    if let Some(name) = self.graph.node_weight(ni) {
                                        if let Some(ep) =
                                            doc.endpoints.iter().find(|e| e.name == *name)
                                        {
                                            if ep.persisted {
                                                color = egui::Color32::from_rgb(0, 170, 90);
                                            } else {
                                                color = egui::Color32::from_rgb(0, 128, 255);
                                            }
                                        }
                                    }
                                }
                            }
                            NodeKind::Pattern => {
                                color = egui::Color32::from_rgb(34, 197, 94);
                            } // emerald
                            NodeKind::Feature => {
                                color = egui::Color32::from_rgb(56, 189, 248);
                            } // sky
                        }
                    }
                    painter.circle_filled(screen_pos, 11.0, color);
                    if let Some(name) = self.graph.node_weight(ni) {
                        painter.text(
                            screen_pos + egui::vec2(12.0, -2.0),
                            egui::Align2::LEFT_CENTER,
                            name,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }

            // Hover tooltip
            if response.hovered() {
                if let Some(pointer) = response.hover_pos() {
                    let mut best: Option<(NodeIndex, f32)> = None;
                    for ni in self.graph.node_indices() {
                        if let Some(pos) = self.node_positions.get(&ni) {
                            let p = to_screen.transform_pos(*pos);
                            let d = p.distance(pointer);
                            if d < 18.0 && (best.is_none() || d < best.unwrap().1) {
                                best = Some((ni, d));
                            }
                        }
                    }
                    if let Some((ni, _)) = best {
                        if let Some(name) = self.graph.node_weight(ni) {
                            egui::show_tooltip_at_pointer(ctx, egui::Id::new("node_tip"), |ui| {
                                ui.label(name);
                            });
                        }
                    }
                }
            }
            // Click to select
            if response.clicked() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let mut best: Option<(NodeIndex, f32)> = None;
                    for ni in self.graph.node_indices() {
                        if let Some(pos) = self.node_positions.get(&ni) {
                            let p = to_screen.transform_pos(*pos);
                            let d = p.distance(pointer);
                            if d < 20.0 && (best.is_none() || d < best.unwrap().1) {
                                best = Some((ni, d));
                            }
                        }
                    }
                    if let Some((ni, _)) = best {
                        self.selected = Some(ni);
                    }
                }
            }
            // Drag to move selected node
            if response.dragged() {
                if let (Some(sel), Some(pointer)) = (self.selected, response.interact_pointer_pos())
                {
                    if let Some(p) = self.node_positions.get_mut(&sel) {
                        // invert transform approximately by linear mapping center
                        // We already draw from logical to screen; approximate inverse by mapping screen delta back
                        // Use small delta movements
                        // For simplicity, set position to inverse map using to_screen inverse
                        let inv = egui::emath::RectTransform::from_to(
                            response.rect,
                            egui::Rect::from_center_size(
                                egui::pos2(0.0, 0.0),
                                egui::vec2(700.0, 700.0),
                            ),
                        );
                        *p = inv.transform_pos(pointer);
                    }
                }
            }
        });
        // Right details panel
        egui::SidePanel::right("right")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.heading("Details");
                if let Some(sel) = self.selected {
                    if let Some(label) = self.graph.node_weight(sel).cloned() {
                        // Determine kind
                        let kind = self.node_kinds.get(&sel).copied().unwrap_or(NodeKind::Endpoint);
                        match kind {
                            NodeKind::Endpoint => {
                                ui.label(format!("Endpoint: {}", label));
                                let current_ep = self
                                    .autodoc
                                    .as_ref()
                                    .and_then(|doc| doc.endpoints.iter().find(|e| e.name == label).cloned());
                                if let Some(ep) = current_ep.clone() {
                                    ui.separator();
                                    ui.label(format!("Persisted: {}", ep.persisted));
                                    ui.label("Description:");
                                    ui.label(&ep.description);
                                    if !ep.parameters.is_empty() {
                                        ui.label(format!("Parameters: {}", ep.parameters.join(", ")));
                                    }
                                    ui.separator();
                                    if ui.button("Approve pattern").clicked() {
                                        let prompt = format!("Approve pattern '{}'", label);
                                let _ = self
                                    .client
                                    .post(format!(
                                        "{}/conversation/{}/prompt",
                                        self.base_url, self.branch_id
                                    ))
                                    .header("Content-Type", "application/json")
                                    .body(
                                        serde_json::to_string(&serde_json::json!({ "prompt": prompt }))
                                            .unwrap(),
                                    )
                                    .send();
                                self.status = "Approve sent".to_string();
                                let _ = std::mem::take(&mut self.events);
                                self.fetch_events();
                            }
                            if let Some(example) = ep.examples.get(0).cloned() {
                                if ui.button("Call example").clicked() {
                                    let _ = self
                                        .client
                                        .post(format!(
                                            "{}/conversation/{}/prompt",
                                            self.base_url, self.branch_id
                                        ))
                                        .header("Content-Type", "application/json")
                                        .body(
                                            serde_json::to_string(&serde_json::json!({ "prompt": example }))
                                                .unwrap(),
                                        )
                                        .send();
                                    self.status = "Call sent".to_string();
                                    let _ = std::mem::take(&mut self.events);
                                    self.fetch_events();
                                }
                            }
                            // Parameterized call UI
                            if !ep.parameters.is_empty() {
                                ui.separator();
                                ui.label("Call with parameters:");
                                let mut args: Vec<(String, String)> = Vec::new();
                                for pname in ep.parameters.iter() {
                                    let mut val = String::new();
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", pname));
                                        ui.text_edit_singleline(&mut val);
                                    });
                                    if !val.is_empty() { args.push((pname.clone(), val)); }
                                }
                                if ui.button("Call API").clicked() {
                                    // Build a prompt like: Call the API 'name' with a='x', b='y'
                                    let mut parts: Vec<String> = Vec::new();
                                    for (k, v) in args { parts.push(format!("{}='{}'", k, v)); }
                                    let arg_str = parts.join(", ");
                                    let prompt = if arg_str.is_empty() {
                                        format!("Call the API '{}'", label)
                                    } else {
                                        format!("Call the API '{}' with {}", label, arg_str)
                                    };
                                    let _ = self
                                        .client
                                        .post(format!(
                                            "{}/conversation/{}/prompt",
                                            self.base_url, self.branch_id
                                        ))
                                        .header("Content-Type", "application/json")
                                        .body(
                                            serde_json::to_string(&serde_json::json!({ "prompt": prompt }))
                                                .unwrap(),
                                        )
                                        .send();
                                    self.status = "Call sent".to_string();
                                    let _ = std::mem::take(&mut self.events);
                                    self.fetch_events();
                                }
                            }
                            }
                            }
                            NodeKind::Pattern | NodeKind::Feature => {
                                ui.label(format!("{}: {}", if kind == NodeKind::Pattern { "Pattern" } else { "Feature" }, label));
                                // Lookup doc path from catalog by matching label prefix removed
                                if let Some(cat) = &self.catalog {
                                    let title = label.trim_start_matches("P: ").trim_start_matches("F: ").to_string();
                                    let mut doc_path: Option<String> = None;
                                    let mut summary: Option<String> = None;
                                    if kind == NodeKind::Pattern {
                                        if let Some(p) = cat.patterns.iter().find(|p| p.title == title) {
                                            doc_path = Some(p.docs_path.clone());
                                            summary = Some(p.summary.clone());
                                        }
                                    } else {
                                        if let Some(f) = cat.features.iter().find(|f| f.title == title) {
                                            doc_path = Some(f.docs_path.clone());
                                            summary = Some(f.summary.clone());
                                        }
                                    }
                                    if let Some(s) = summary { ui.label(s); }
                                    if let Some(path) = doc_path.clone() {
                                        if ui.button("Open doc").clicked() {
                                            let base = std::env::var("ENGINE_DOCS_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
                                            let url = format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'));
                                            let _ = if cfg!(target_os = "macos") {
                                                std::process::Command::new("open").arg(&url).spawn()
                                            } else if cfg!(target_os = "windows") {
                                                std::process::Command::new("cmd").args(["/C", "start", &url]).spawn()
                                            } else {
                                                std::process::Command::new("xdg-open").arg(&url).spawn()
                                            };
                                            self.status = "Opened doc".to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    ui.label("Select a node to see details.");
                }
                ui.separator();
                ui.label("Legend:");
                ui.label("Green: persisted endpoint; Blue: ephemeral; Emerald: pattern; Sky: feature");
                ui.separator();
                // Onboard context (UI-only stub): suggest a doc path to create
                ui.collapsing("Onboard context", |ui| {
                    ui.label("Paste any context below and choose a suggested doc path. Create the file under docs/ and serve via open_canvas.sh.");
                    ui.text_edit_multiline(&mut self.context_input);
                    let suggested = if !self.context_input.trim().is_empty() {
                        let slug = slugify(&self.context_input);
                        format!("docs/contexts/{}.html", slug)
                    } else { "docs/contexts/<slug>.html".to_string() };
                    ui.horizontal(|ui| {
                        ui.label(format!("Suggested: {}", suggested));
                    });
                });
                ui.separator();
                // Templates Panel (manual actions; no auto-refresh)
                if self.show_templates {
                    ui.heading("Templates");
                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        ui.text_edit_singleline(&mut self.template_search);
                    });
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for t in TEMPLATE_SPECS.iter() {
                            if !self.template_search.trim().is_empty() {
                                let q = self.template_search.to_lowercase();
                                let name_hit = t.name.to_lowercase().contains(&q);
                                let desc_hit = t.description.to_lowercase().contains(&q);
                                if !(name_hit || desc_hit) { continue; }
                            }
                            ui.group(|ui| {
                                ui.label(egui::RichText::new(t.name).strong());
                                ui.label(t.description);
                                ui.horizontal(|ui| {
                                    // Define
                                    if ui.button("Define").clicked() {
                                        let prompt = (t.define_prompt)(t);
                                        let _ = self
                                            .client
                                            .post(format!(
                                                "{}/conversation/{}/prompt",
                                                self.base_url, self.branch_id
                                            ))
                                            .header("Content-Type", "application/json")
                                            .body(
                                                serde_json::to_string(&serde_json::json!({ "prompt": prompt }))
                                                    .unwrap(),
                                            )
                                            .send();
                                        self.status = format!("Define '{}' sent", t.name);
                                    }
                                    // Approve
                                    if ui.button("Approve").clicked() {
                                        let prompt = format!("Approve pattern '{}'", t.name);
                                        let _ = self
                                            .client
                                            .post(format!(
                                                "{}/conversation/{}/prompt",
                                                self.base_url, self.branch_id
                                            ))
                                            .header("Content-Type", "application/json")
                                            .body(
                                                serde_json::to_string(&serde_json::json!({ "prompt": prompt }))
                                                    .unwrap(),
                                            )
                                            .send();
                                        self.status = format!("Approve '{}' sent", t.name);
                                    }
                                });
                                // Call with params (if any)
                                if !t.params.is_empty() {
                                    ui.separator();
                                    ui.label("Call parameters:");
                                    let entry = self
                                        .template_inputs
                                        .entry(t.name.to_string())
                                        .or_insert_with(|| HashMap::new());
                                    for p in t.params.iter() {
                                        let val = entry.entry(p.key.to_string()).or_insert_with(String::new);
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}:", p.key));
                                            ui.text_edit_singleline(val);
                                        });
                                    }
                                }
                                if ui.button("Call").clicked() {
                                    // Build prompt: Call the API 'name' with k='v', ...
                                    let mut parts: Vec<String> = Vec::new();
                                    if let Some(map) = self.template_inputs.get(t.name) {
                                        for p in t.params.iter() {
                                            if let Some(v) = map.get(p.key) {
                                                if !v.is_empty() { parts.push(format!("{}='{}'", p.key, v)); }
                                            }
                                        }
                                    }
                                    let arg_str = parts.join(", ");
                                    let prompt = if arg_str.is_empty() {
                                        format!("Call the API '{}'", t.name)
                                    } else {
                                        format!("Call the API '{}' with {}", t.name, arg_str)
                                    };
                                    let _ = self
                                        .client
                                        .post(format!(
                                            "{}/conversation/{}/prompt",
                                            self.base_url, self.branch_id
                                        ))
                                        .header("Content-Type", "application/json")
                                        .body(
                                            serde_json::to_string(&serde_json::json!({ "prompt": prompt }))
                                                .unwrap(),
                                        )
                                        .send();
                                    self.status = format!("Call '{}' sent", t.name);
                                }
                            });
                            ui.add_space(6.0);
                        }
                    });
                }
            });
    }
}

fn load_branch_id_from_file() -> Option<String> {
    let path = std::path::Path::new("out_one_engine/branch_id.txt");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn main() -> eframe::Result<()> {
    let base_url =
        std::env::var("ENGINE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:7777".to_string());
    let branch_id = std::env::var("ENGINE_BRANCH_ID")
        .ok()
        .or_else(load_branch_id_from_file)
        .unwrap_or_default();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "One Engine – Graph UI",
        options,
        Box::new(|_cc| Box::new(GraphUiApp::new(base_url, branch_id))),
    )
}
