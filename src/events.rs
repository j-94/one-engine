use serde::Serialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Payload delivered to SSE subscribers.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: String,
    pub data: serde_json::Value,
    pub id: Option<String>,
}

/// Shared event stream for structured consciousness updates.
#[derive(Clone)]
pub struct EventStream {
    sender: broadcast::Sender<SseEvent>,
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new(256)
    }
}

impl EventStream {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.sender.subscribe()
    }

    pub fn emit_value(&self, event: impl Into<String>, value: serde_json::Value) {
        let message = SseEvent {
            event: event.into(),
            data: value,
            id: None,
        };
        let _ = self.sender.send(message);
    }

    pub fn emit_json<T>(&self, event: impl Into<String>, payload: &T)
    where
        T: Serialize,
    {
        match serde_json::to_value(payload) {
            Ok(value) => self.emit_value(event, value),
            Err(err) => {
                let _ = self.sender.send(SseEvent {
                    event: "cog.error".to_string(),
                    data: json!({
                        "message": "failed to serialize event payload",
                        "error": err.to_string(),
                    }),
                    id: None,
                });
            }
        }
    }

    pub fn emit_cog_node(
        &self,
        id: impl Into<String>,
        kind: CognitiveNodeKind,
        label: impl Into<String>,
    ) {
        self.emit_value(
            "cog.node",
            json!({
                "id": id.into(),
                "kind": kind.as_str(),
                "label": label.into(),
                "ts": current_timestamp_ms(),
            }),
        );
    }

    pub fn emit_cog_edge(
        &self,
        src: impl Into<String>,
        dst: impl Into<String>,
        rel: CognitiveEdgeRelation,
    ) {
        self.emit_value(
            "cog.edge",
            json!({
                "src": src.into(),
                "dst": dst.into(),
                "rel": rel.as_str(),
            }),
        );
    }

    pub fn emit_cog_frame(
        &self,
        id: impl Into<String>,
        phase: CognitivePhase,
        uncertainty: f32,
        tokens_used: u32,
        notes: Vec<String>,
    ) {
        self.emit_value(
            "cog.frame",
            json!({
                "id": id.into(),
                "phase": phase.as_str(),
                "uncertainty": uncertainty,
                "tokens_used": tokens_used,
                "notes": notes,
            }),
        );
    }

    pub fn emit_cog_metric(&self, name: impl Into<String>, value: f64) {
        self.emit_value(
            "cog.metric",
            json!({
                "name": name.into(),
                "value": value,
                "ts": current_timestamp_ms(),
            }),
        );
    }

    pub fn next_id() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CognitiveNodeKind {
    Goal,
    Hypothesis,
    Belief,
    Critique,
    Plan,
    Constraint,
    Observation,
}

impl CognitiveNodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Goal => "Goal",
            Self::Hypothesis => "Hypothesis",
            Self::Belief => "Belief",
            Self::Critique => "Critique",
            Self::Plan => "Plan",
            Self::Constraint => "Constraint",
            Self::Observation => "Observation",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CognitiveEdgeRelation {
    Supports,
    Contradicts,
    DependsOn,
    Refines,
    Follows,
}

impl CognitiveEdgeRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::DependsOn => "depends_on",
            Self::Refines => "refines",
            Self::Follows => "follows",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CognitivePhase {
    Perceive,
    Plan,
    Act,
    Reflect,
}

impl CognitivePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Perceive => "perceive",
            Self::Plan => "plan",
            Self::Act => "act",
            Self::Reflect => "reflect",
        }
    }
}

fn current_timestamp_ms() -> u64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    duration.as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_strings_match_contract() {
        assert_eq!(CognitiveNodeKind::Goal.as_str(), "Goal");
        assert_eq!(CognitiveNodeKind::Observation.as_str(), "Observation");
    }

    #[test]
    fn edge_relation_strings_match_contract() {
        assert_eq!(CognitiveEdgeRelation::DependsOn.as_str(), "depends_on");
        assert_eq!(CognitiveEdgeRelation::Refines.as_str(), "refines");
    }

    #[test]
    fn phase_strings_match_contract() {
        assert_eq!(CognitivePhase::Act.as_str(), "act");
    }
}
