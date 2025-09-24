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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchState {
    pub branch_id: Uuid,
    pub label: Option<String>,
    pub created_at: SystemTime,
    pub created_from: BranchOrigin,
    pub generated_apis: HashMap<String, GeneratedApi>,
    pub events: Vec<BranchEvent>,
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
            ApiLogic::Static { value } => Ok(value.clone()),
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
