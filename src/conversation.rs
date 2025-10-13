use crate::branch::{ApiLogic, ApiParameter, BranchEvent, BranchManager, GeneratedApi};
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
}

impl ConversationService {
    pub fn new(branches: BranchManager) -> Self {
        Self {
            branches,
            concurrency_guard: Arc::new(Semaphore::new(32)),
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

        match instruction {
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
                Ok(ConversationEffect::ApiCreated { api })
            }
            ParsedInstruction::CallApi(spec) => {
                let api = self
                    .branches
                    .get_api(branch_id, &spec.name)
                    .await
                    .ok_or_else(|| anyhow!("API '{}' not found in branch", spec.name))?;
                // handle counter stateful template
                let output = match api.logic {
                    crate::branch::ApiLogic::Counter => {
                        let count = self.branches.inc_counter(branch_id, &spec.name).await;
                        count.to_string()
                    }
                    _ => api.execute(&spec.arguments)?,
                };
                // Record call and response
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ApiCalled {
                            name: spec.name.clone(),
                        },
                    )
                    .await;
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ApiResponse {
                            name: spec.name.clone(),
                            output: output.clone(),
                        },
                    )
                    .await;
                // Data-flow inference: if previous output equals any current argument value
                if let Some((from_api, last_out)) = self.branches.get_last_output(branch_id).await {
                    if spec.arguments.values().any(|v| v == &last_out) {
                        self.branches
                            .record_data_flow(branch_id, from_api, spec.name.clone())
                            .await;
                    }
                }
                // Update last output
                self.branches
                    .set_last_output(branch_id, spec.name.clone(), output.clone())
                    .await;

                Ok(ConversationEffect::ApiResponse {
                    name: spec.name,
                    output,
                })
            }
            ParsedInstruction::ApprovePattern { name } => {
                // Mark API as persisted if it exists, then record approval intent
                let _ = self.branches.persist_api(branch_id, &name).await;
                self.branches
                    .record_event(
                        branch_id,
                        BranchEvent::ParsedIntent {
                            description: format!("approval: {}", name),
                        },
                    )
                    .await;
                Ok(ConversationEffect::ApprovalRecorded { name })
            }
            ParsedInstruction::Unknown { original } => {
                Ok(ConversationEffect::Unknown { prompt: original })
            }
        }
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
        BehavioralHint::Custom(body) => {
            // Map known template names to logic variants
            match spec.name.to_lowercase().as_str() {
                "uppercase" => ApiLogic::Uppercase,
                "concat" => ApiLogic::Concat,
                "slugify" => ApiLogic::Slugify,
                "sum" => ApiLogic::Sum,
                "counter" => ApiLogic::Counter,
                _ => ApiLogic::Custom { body: body.clone() },
            }
        }
    };

    Ok(GeneratedApi {
        name: spec.name.clone(),
        description: spec.description.clone(),
        parameters,
        logic,
        persisted: matches!(spec.persistence, PersistenceDirective::Persist),
    })
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

    #[tokio::test]
    async fn creates_and_calls_echo_api() {
        let service = ConversationService::new(BranchManager::new());
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
}
