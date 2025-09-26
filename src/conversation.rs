use crate::branch::{ApiLogic, ApiParameter, BranchEvent, BranchManager, GeneratedApi};
use crate::events::{CognitiveEdgeRelation, CognitiveNodeKind, CognitivePhase, EventStream};
use crate::parser::{parse_instruction, BehavioralHint, ParsedInstruction, PersistenceDirective};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Coordinates between the natural language parser and the branch manager.
#[derive(Clone)]
pub struct ConversationService {
    branches: BranchManager,
    concurrency_guard: Arc<Semaphore>,
    events: EventStream,
}

impl ConversationService {
    pub fn new(branches: BranchManager, events: EventStream) -> Self {
        Self {
            branches,
            concurrency_guard: Arc::new(Semaphore::new(32)),
            events,
        }
    }

    /// Create a fresh branch for a new conversational thread.
    pub async fn start_session(&self, label: Option<String>) -> Uuid {
        self.branches.create_branch(label).await
    }

    /// Parse and execute a prompt within the given branch.
    pub async fn process_prompt(
        &self,
        branch_id: Uuid,
        prompt: &str,
    ) -> Result<ConversationEffect> {
        let _permit = self.concurrency_guard.acquire().await?;

        let frame_id = format!("branch-{branch_id}");
        let prompt_tokens = prompt.split_whitespace().count() as u32;
        let prompt_node_id = format!("prompt-{}", EventStream::next_id());

        self.events.emit_cog_frame(
            frame_id.clone(),
            CognitivePhase::Perceive,
            0.45,
            prompt_tokens,
            vec![
                format!("prompt: {prompt}"),
                format!("branch_id: {branch_id}"),
            ],
        );
        self.events.emit_cog_node(
            prompt_node_id.clone(),
            CognitiveNodeKind::Observation,
            prompt.to_string(),
        );
        self.events.emit_cog_metric("search_depth", 1.0);

        self.branches
            .record_event(
                branch_id,
                BranchEvent::Prompt {
                    content: prompt.to_string(),
                },
            )
            .await;

        let instruction = parse_instruction(prompt);
        self.branches
            .record_event(
                branch_id,
                BranchEvent::ParsedIntent {
                    description: format!("{:?}", instruction),
                },
            )
            .await;

        let intent_node_id = format!("intent-{}", EventStream::next_id());
        let (intent_kind, intent_label) = describe_instruction(&instruction);
        self.events
            .emit_cog_node(intent_node_id.clone(), intent_kind, intent_label.clone());
        self.events.emit_cog_edge(
            prompt_node_id.clone(),
            intent_node_id.clone(),
            CognitiveEdgeRelation::Refines,
        );
        self.events.emit_cog_frame(
            frame_id.clone(),
            CognitivePhase::Plan,
            0.35,
            prompt_tokens + intent_label.split_whitespace().count() as u32,
            vec![format!("intent: {intent_label}")],
        );

        let effect = match instruction {
            ParsedInstruction::CreateApi(spec) => {
                let api = convert_spec_to_api(&spec)?;
                self.branches.upsert_api(branch_id, api.clone()).await;
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ApiGenerated {
                            name: api.name.clone(),
                        },
                    )
                    .await;

                let result_node_id = format!("result-{}", EventStream::next_id());
                self.events.emit_cog_node(
                    result_node_id.clone(),
                    CognitiveNodeKind::Plan,
                    format!("Created API {}", api.name),
                );
                self.events.emit_cog_edge(
                    intent_node_id,
                    result_node_id,
                    CognitiveEdgeRelation::Follows,
                );
                self.events.emit_cog_frame(
                    frame_id.clone(),
                    CognitivePhase::Act,
                    0.25,
                    api.description.split_whitespace().count() as u32,
                    vec![format!("api: {}", api.name)],
                );
                self.events.emit_cog_metric("alt_count", 1.0);
                self.events.emit_cog_frame(
                    frame_id,
                    CognitivePhase::Reflect,
                    0.2,
                    0,
                    vec!["api definition recorded".to_string()],
                );

                ConversationEffect::ApiCreated { api }
            }
            ParsedInstruction::CallApi(spec) => {
                let api = self
                    .branches
                    .get_api(branch_id, &spec.name)
                    .await
                    .ok_or_else(|| anyhow!("API '{}' not found in branch", spec.name))?;
                let output = api.execute(&spec.arguments)?;
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ApiCalled {
                            name: spec.name.clone(),
                        },
                    )
                    .await;

                let result_node_id = format!("result-{}", EventStream::next_id());
                self.events.emit_cog_node(
                    result_node_id.clone(),
                    CognitiveNodeKind::Belief,
                    format!("API {} returned", spec.name),
                );
                self.events.emit_cog_edge(
                    intent_node_id,
                    result_node_id.clone(),
                    CognitiveEdgeRelation::Supports,
                );
                self.events.emit_cog_frame(
                    frame_id.clone(),
                    CognitivePhase::Act,
                    0.3,
                    spec.arguments.len() as u32,
                    vec![format!("api call: {}", spec.name)],
                );
                self.events.emit_cog_metric("repeat_score", 0.0);
                self.events.emit_cog_frame(
                    frame_id,
                    CognitivePhase::Reflect,
                    0.2,
                    output.split_whitespace().count() as u32,
                    vec![format!("output: {}", truncate_for_notes(&output))],
                );

                ConversationEffect::ApiResponse {
                    name: spec.name,
                    output,
                }
            }
            ParsedInstruction::ApprovePattern { name } => {
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ParsedIntent {
                            description: format!("approval: {}", name),
                        },
                    )
                    .await;

                let result_node_id = format!("result-{}", EventStream::next_id());
                self.events.emit_cog_node(
                    result_node_id.clone(),
                    CognitiveNodeKind::Constraint,
                    format!("Approved pattern {name}"),
                );
                self.events.emit_cog_edge(
                    intent_node_id,
                    result_node_id,
                    CognitiveEdgeRelation::DependsOn,
                );
                self.events.emit_cog_frame(
                    frame_id.clone(),
                    CognitivePhase::Act,
                    0.3,
                    name.split_whitespace().count() as u32,
                    vec![format!("approval: {name}")],
                );
                self.events.emit_cog_frame(
                    frame_id,
                    CognitivePhase::Reflect,
                    0.18,
                    0,
                    vec!["approval recorded".to_string()],
                );

                ConversationEffect::ApprovalRecorded { name }
            }
            ParsedInstruction::Unknown { original } => {
                self.events.emit_cog_frame(
                    frame_id,
                    CognitivePhase::Reflect,
                    0.5,
                    original.split_whitespace().count() as u32,
                    vec!["instruction unresolved".to_string()],
                );

                ConversationEffect::Unknown { prompt: original }
            }
        };

        Ok(effect)
    }
}

fn convert_spec_to_api(spec: &crate::parser::CreateApiSpec) -> Result<GeneratedApi> {
    if spec.parameters.is_empty() {
        return Err(anyhow!("At least one parameter is required"));
    }

    let parameters = spec
        .parameters
        .iter()
        .map(|param| ApiParameter {
            name: param.name.clone(),
            param_type: param.param_type.clone(),
            description: param.description.clone(),
        })
        .collect::<Vec<_>>();

    let logic = match &spec.behavioral_hint {
        BehavioralHint::Echo | BehavioralHint::PassThrough => ApiLogic::Echo {
            key: parameters.first().map(|p| p.name.clone()),
        },
        BehavioralHint::Custom(body) => ApiLogic::Custom { body: body.clone() },
    };

    Ok(GeneratedApi {
        name: spec.name.clone(),
        description: spec.description.clone(),
        parameters,
        logic,
        persisted: matches!(spec.persistence, PersistenceDirective::Persist),
    })
}

fn describe_instruction(instruction: &ParsedInstruction) -> (CognitiveNodeKind, String) {
    match instruction {
        ParsedInstruction::CreateApi(spec) => {
            (CognitiveNodeKind::Plan, format!("Create API {}", spec.name))
        }
        ParsedInstruction::CallApi(spec) => {
            (CognitiveNodeKind::Goal, format!("Call API {}", spec.name))
        }
        ParsedInstruction::ApprovePattern { name } => (
            CognitiveNodeKind::Constraint,
            format!("Approve pattern {name}"),
        ),
        ParsedInstruction::Unknown { original } => (
            CognitiveNodeKind::Hypothesis,
            format!("Interpret prompt: {original}"),
        ),
    }
}

fn truncate_for_notes(text: &str) -> String {
    const MAX: usize = 64;
    if text.len() <= MAX {
        return text.to_string();
    }
    let slice = text.chars().take(MAX - 1).collect::<String>();
    format!("{slice}…")
}

/// Result of a conversational turn.
#[derive(Debug, Serialize, Clone)]
pub enum ConversationEffect {
    ApiCreated { api: GeneratedApi },
    ApiResponse { name: String, output: String },
    ApprovalRecorded { name: String },
    Unknown { prompt: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn creates_and_calls_echo_api() {
        let events = EventStream::default();
        let service = ConversationService::new(BranchManager::new(), events);
        let branch_id = service.start_session(Some("test".to_string())).await;

        let define_effect = service
            .process_prompt(
                branch_id,
                "Define a simple API named 'echo' that accepts a single parameter 'text' and returns it unmodified.",
            )
            .await
            .unwrap();

        let api_name = match define_effect {
            ConversationEffect::ApiCreated { api } => api.name,
            _ => panic!("expected api creation"),
        };

        let call_effect = service
            .process_prompt(branch_id, "Call the API 'echo' with text='Hello'")
            .await
            .unwrap();

        if let ConversationEffect::ApiResponse { name, output } = call_effect {
            assert_eq!(name, api_name);
            assert_eq!(output, "Hello");
        } else {
            panic!("expected api response");
        }
    }

    #[tokio::test]
    async fn emits_cognitive_events_for_prompt() {
        let events = EventStream::default();
        let mut rx = events.subscribe();
        let service = ConversationService::new(BranchManager::new(), events);
        let branch_id = service.start_session(None).await;

        let _ = service
            .process_prompt(branch_id, "Call the API 'echo' with text='Hello'")
            .await
            .unwrap();

        let mut saw_node = false;
        for _ in 0..6 {
            if let Ok(message) = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
                if let Ok(event) = message {
                    if event.event == "cog.node" {
                        saw_node = true;
                        break;
                    }
                }
            }
        }

        assert!(saw_node, "expected at least one cog.node event");
    }
}
