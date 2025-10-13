use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Manager that tracks evolving branches of the engine's consciousness.
#[derive(Clone, Default)]
pub struct BranchManager {
    inner: Arc<RwLock<HashMap<Uuid, BranchState>>>,
}

impl BranchManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new branch derived from the main consciousness.
    pub async fn create_branch(&self, label: Option<String>) -> Uuid {
        let branch_id = Uuid::new_v4();
        let state = BranchState {
            branch_id,
            label,
            created_at: SystemTime::now(),
            created_from: BranchOrigin::Main,
            generated_apis: HashMap::new(),
            events: Vec::new(),
            last_output: None,
            last_api: None,
            api_counters: HashMap::new(),
        };

        let mut branches = self.inner.write().await;
        branches.insert(branch_id, state);
        branch_id
    }

    /// Record a branching event (prompt, action, etc.).
    pub async fn record_event(&self, branch_id: Uuid, event: BranchEvent) {
        if let Some(branch) = self.inner.write().await.get_mut(&branch_id) {
            branch.events.push(event);
        }
    }

    /// Store or update a generated API inside a branch.
    pub async fn upsert_api(&self, branch_id: Uuid, api: GeneratedApi) {
        if let Some(branch) = self.inner.write().await.get_mut(&branch_id) {
            branch.generated_apis.insert(api.name.clone(), api);
        }
    }

    /// Mark an existing API as persisted (approved) in a branch
    pub async fn persist_api(&self, branch_id: Uuid, name: &str) -> bool {
        let mut guard = self.inner.write().await;
        if let Some(branch) = guard.get_mut(&branch_id) {
            if let Some(api) = branch.generated_apis.get_mut(name) {
                api.persisted = true;
                return true;
            }
        }
        false
    }

    pub async fn get_api(&self, branch_id: Uuid, name: &str) -> Option<GeneratedApi> {
        self.inner
            .read()
            .await
            .get(&branch_id)
            .and_then(|branch| branch.generated_apis.get(name).cloned())
    }

    #[allow(dead_code)]
    pub async fn list_branch_ids(&self) -> Vec<Uuid> {
        self.inner.read().await.keys().copied().collect()
    }

    pub async fn snapshot(&self, branch_id: Uuid) -> Option<BranchState> {
        self.inner.read().await.get(&branch_id).cloned()
    }

    pub async fn set_last_output(&self, branch_id: Uuid, api_name: String, output: String) {
        if let Some(branch) = self.inner.write().await.get_mut(&branch_id) {
            branch.last_api = Some(api_name);
            branch.last_output = Some(output);
        }
    }

    pub async fn get_last_output(&self, branch_id: Uuid) -> Option<(String, String)> {
        let guard = self.inner.read().await;
        if let Some(b) = guard.get(&branch_id) {
            if let (Some(a), Some(o)) = (b.last_api.clone(), b.last_output.clone()) {
                return Some((a, o));
            }
        }
        None
    }

    pub async fn inc_counter(&self, branch_id: Uuid, api_name: &str) -> i64 {
        let mut guard = self.inner.write().await;
        if let Some(b) = guard.get_mut(&branch_id) {
            let e = b.api_counters.entry(api_name.to_string()).or_insert(0);
            *e += 1;
            return *e;
        }
        0
    }

    pub async fn record_data_flow(&self, branch_id: Uuid, from: String, to: String) {
        self.record_event(branch_id, BranchEvent::DataFlow { from, to }).await;
    }

    /// Generate autodoc for a branch by inspecting generated APIs and prompts that call them
    pub async fn generate_autodoc(&self, branch_id: Uuid) -> Option<AutoDoc> {
        let branches = self.inner.read().await;
        let branch = branches.get(&branch_id)?;

        let mut endpoints = Vec::new();
        // Precollect call prompts for quick lookup
        let mut prompts: Vec<String> = Vec::new();
        for ev in &branch.events {
            if let BranchEvent::Prompt { content } = ev {
                prompts.push(content.clone());
            }
        }

        for (_k, api) in &branch.generated_apis {
            // Find example prompts that call this API
            let name = &api.name;
            let examples: Vec<String> = prompts
                .iter()
                .filter(|p| p.contains(&format!("Call the API '{}'", name)))
                .cloned()
                .collect();

            let parameters = api.parameters.iter().map(|p| p.name.clone()).collect();
            endpoints.push(EndpointDoc {
                name: api.name.clone(),
                description: api.description.clone(),
                parameters,
                persisted: api.persisted,
                examples,
            });
        }

        Some(AutoDoc {
            branch_id: branch.branch_id,
            label: branch.label.clone(),
            endpoints,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchState {
    pub branch_id: Uuid,
    pub label: Option<String>,
    pub created_at: SystemTime,
    pub created_from: BranchOrigin,
    pub generated_apis: HashMap<String, GeneratedApi>,
    pub events: Vec<BranchEvent>,
    // Simple conversational memory for data-flow inference
    pub last_output: Option<String>,
    pub last_api: Option<String>,
    // Simple per-API counters (stateful templates)
    pub api_counters: HashMap<String, i64>,
}

/// Autodocumentation structures derived from branch state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDoc {
    pub branch_id: Uuid,
    pub label: Option<String>,
    pub endpoints: Vec<EndpointDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDoc {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub persisted: bool,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BranchOrigin {
    Main,
    Branch(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BranchEvent {
    Prompt { content: String },
    ParsedIntent { description: String },
    ApiGenerated { name: String },
    ApiCalled { name: String },
    ApiResponse { name: String, output: String },
    DataFlow { from: String, to: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedApi {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ApiParameter>,
    pub logic: ApiLogic,
    pub persisted: bool,
}

impl GeneratedApi {
    pub fn execute(&self, args: &HashMap<String, String>) -> anyhow::Result<String> {
        match &self.logic {
            ApiLogic::Echo { key } => {
                let key = key.as_deref().unwrap_or("text");
                if let Some(value) = args.get(key) {
                    Ok(value.clone())
                } else if let Some(first) = self.parameters.first() {
                    Ok(args
                        .get(&first.name)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()))
                } else {
                    Ok(String::new())
                }
            }
            ApiLogic::Uppercase => {
                let key = self.parameters.first().map(|p| p.name.as_str()).unwrap_or("text");
                let input = args.get(key).cloned().unwrap_or_default();
                Ok(input.to_uppercase())
            }
            ApiLogic::Concat => {
                let a = args.get("a").cloned().unwrap_or_default();
                let b = args.get("b").cloned().unwrap_or_default();
                Ok(format!("{}{}", a, b))
            }
            ApiLogic::Slugify => {
                let key = self.parameters.first().map(|p| p.name.as_str()).unwrap_or("text");
                let input = args.get(key).cloned().unwrap_or_default();
                let slug = input
                    .to_lowercase()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else if c.is_whitespace() || c == '-' { '-' } else { '-' })
                    .collect::<String>()
                    .trim_matches('-')
                    .to_string();
                // collapse multiple dashes
                let mut collapsed = String::new();
                let mut prev_dash = false;
                for ch in slug.chars() {
                    if ch == '-' {
                        if !prev_dash { collapsed.push('-'); }
                        prev_dash = true;
                    } else {
                        collapsed.push(ch);
                        prev_dash = false;
                    }
                }
                Ok(collapsed)
            }
            ApiLogic::Sum => {
                let a = args.get("a").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                let b = args.get("b").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                Ok((a + b).to_string())
            }
            ApiLogic::Static { value } => Ok(value.clone()),
            ApiLogic::Counter => {
                // Counter is handled in ConversationService to maintain state; fallback output here
                Ok("counter".to_string())
            }
            ApiLogic::Custom { body } => Ok(format!(
                "logic-not-implemented: {{name: {}, body: {}}}",
                self.name, body
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub param_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiLogic {
    Echo { key: Option<String> },
    Uppercase,
    Concat,
    Slugify,
    Sum,
    Counter,
    Static { value: String },
    Custom { body: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn branch_records_api_definition() {
        let manager = BranchManager::new();
        let branch_id = manager.create_branch(Some("test".to_string())).await;

        let api = GeneratedApi {
            name: "echo".to_string(),
            description: "Echo API".to_string(),
            parameters: vec![ApiParameter {
                name: "text".to_string(),
                param_type: None,
                description: None,
            }],
            logic: ApiLogic::Echo {
                key: Some("text".to_string()),
            },
            persisted: false,
        };

        manager.upsert_api(branch_id, api.clone()).await;

        let stored = manager.get_api(branch_id, "echo").await;
        assert!(stored.is_some());
        assert_eq!(
            stored
                .unwrap()
                .execute(&HashMap::from([("text".to_string(), "hello".to_string())]))
                .unwrap(),
            "hello"
        );
    }
}
