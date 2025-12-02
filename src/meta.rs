use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Lightweight catalogue that materialises Meta³ canons from disk.
#[derive(Debug, Clone)]
pub struct MetaCatalog {
    canons: HashMap<String, MetaCanon>,
    root: PathBuf,
}

impl MetaCatalog {
    /// Load canons from a directory (`*.yaml`). Missing directories produce an empty catalogue.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        if !root.exists() {
            return Ok(Self::empty_with_root(root));
        }

        let mut canons = HashMap::new();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
                != Some(true)
            {
                continue;
            }

            let raw_yaml = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    warn!("failed to read canon file {}: {}", path.display(), err);
                    continue;
                }
            };

            let raw: Value = match serde_yaml::from_str(&raw_yaml) {
                Ok(value) => value,
                Err(err) => {
                    warn!("failed to parse canon {}: {}", path.display(), err);
                    continue;
                }
            };

            match MetaCanon::from_value(raw, path.clone()) {
                Ok(canon) => {
                    canons.insert(canon.name.clone(), canon);
                }
                Err(err) => {
                    warn!("skipping canon {}: {}", path.display(), err);
                }
            }
        }

        Ok(Self { canons, root })
    }

    fn empty_with_root(root: PathBuf) -> Self {
        Self {
            canons: HashMap::new(),
            root,
        }
    }

    /// Construct an empty catalogue.
    pub fn empty() -> Self {
        Self::empty_with_root(PathBuf::new())
    }

    /// All canon summaries sorted by name.
    pub fn summaries(&self) -> Vec<MetaCanonSummary> {
        let mut items: Vec<_> = self.canons.values().map(MetaCanon::summary).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        items
    }

    /// Fetch a canon by name.
    pub fn get(&self, name: &str) -> Option<&MetaCanon> {
        self.canons.get(name)
    }

    /// Directory used for discovery (useful for debugging).
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Action preview extracted from a canon.
#[derive(Debug, Clone, Serialize)]
pub struct ActionPreview {
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub action_type: Option<String>,
}

/// Summary used for lightweight catalogue calls.
#[derive(Debug, Clone, Serialize)]
pub struct MetaCanonSummary {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub intent: Option<String>,
    pub scope: Option<String>,
    pub system_layer: Option<String>,
    pub source_refs: Vec<String>,
}

/// Canon with additional metadata for detailed inspection.
#[derive(Debug, Clone)]
pub struct MetaCanon {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub intent: Option<String>,
    pub scope: Option<String>,
    pub system_layer: Option<String>,
    pub source_refs: Vec<String>,
    raw: Value,
    path: PathBuf,
}

impl MetaCanon {
    fn from_value(raw: Value, path: PathBuf) -> Result<Self> {
        let name = raw
            .get("name")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .context("canon requires `name`")?;
        let version = raw
            .get("version")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .context("canon requires `version`")?;
        let description = raw
            .get("description")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let intent = raw
            .get("intent")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let scope = raw
            .get("scope")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let system_layer = raw
            .get("system_layer")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let source_refs = raw
            .get("source_refs")
            .and_then(Value::as_sequence)
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            version,
            description,
            intent,
            scope,
            system_layer,
            source_refs,
            raw,
            path,
        })
    }

    /// Generate a reusable summary.
    pub fn summary(&self) -> MetaCanonSummary {
        MetaCanonSummary {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
            intent: self.intent.clone(),
            scope: self.scope.clone(),
            system_layer: self.system_layer.clone(),
            source_refs: self.source_refs.clone(),
        }
    }

    /// Preview the declared actions (`label`, `type`).
    pub fn action_preview(&self) -> Vec<ActionPreview> {
        let mut previews = Vec::new();
        if let Some(actions) = self.raw.get("actions").and_then(Value::as_sequence) {
            for action in actions {
                if let Some(mapping) = action.as_mapping() {
                    let action_type = mapping_get(mapping, "type").and_then(Value::as_str);
                    let label = mapping_get(mapping, "label").and_then(Value::as_str);
                    previews.push(ActionPreview {
                        label: label.map(|s| s.to_string()),
                        action_type: action_type.map(|s| s.to_string()),
                    });
                }
            }
        }
        previews
    }

    /// Raw canon as JSON (useful for downstream tooling).
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.raw).unwrap_or_else(|_| serde_json::Value::Null)
    }

    /// Source path on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(&Value::String(key.to_string()))
}
