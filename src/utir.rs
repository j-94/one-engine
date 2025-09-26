use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Universal Task IR - The complete syntactic vocabulary for safe operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtirDocument {
    pub task_id: String,
    pub description: String,
    pub operations: Vec<Operation>,
    pub policy: Option<Policy>,
    pub bits_tracking: Option<BitsTracking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub gamma_gate: f64,
    pub time_ms: u64,
    pub max_risk: f64,
    pub tiny_diff_loc: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitsTracking {
    pub track_all: bool,
    pub custom_bits: HashMap<String, String>,
}

/// Core operation types - the agent's limited vocabulary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    #[serde(rename = "shell")]
    Shell {
        command: String,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default)]
        working_dir: Option<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        allow_network: bool,
        #[serde(default = "default_true")]
        capture_output: bool,
    },

    #[serde(rename = "fs.read")]
    FsRead {
        path: String,
        #[serde(default = "default_encoding")]
        encoding: String,
        #[serde(default = "default_max_size")]
        max_size: String,
    },

    #[serde(rename = "fs.write")]
    FsWrite {
        path: String,
        content: String,
        #[serde(default = "default_mode")]
        mode: String,
        #[serde(default)]
        create_dirs: bool,
    },

    #[serde(rename = "http.get")]
    HttpGet {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default = "default_max_response")]
        max_response_size: String,
    },

    #[serde(rename = "git.patch")]
    GitPatch {
        repo_path: String,
        patch_content: String,
        commit_message: String,
        author: String,
    },

    #[serde(rename = "assert.file_exists")]
    AssertFileExists { path: String },

    #[serde(rename = "assert.shell_success")]
    AssertShellSuccess {
        command: String,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default)]
        expected_output: Option<String>,
    },

    #[serde(rename = "sequence")]
    Sequence { steps: Vec<Operation> },

    #[serde(rename = "parallel")]
    Parallel {
        steps: Vec<Operation>,
        #[serde(default = "default_concurrency")]
        max_concurrency: u32,
    },

    #[serde(rename = "conditional")]
    Conditional {
        condition: Box<Operation>,
        then_op: Box<Operation>,
        else_op: Option<Box<Operation>>,
    },

    #[serde(rename = "retry")]
    Retry {
        operation: Box<Operation>,
        #[serde(default = "default_retry_attempts")]
        max_attempts: u32,
        #[serde(default = "default_backoff")]
        backoff: String,
    },
}

// Default functions for serialization
fn default_timeout() -> String {
    "30s".to_string()
}
fn default_encoding() -> String {
    "utf-8".to_string()
}
fn default_max_size() -> String {
    "10MB".to_string()
}
fn default_mode() -> String {
    "0644".to_string()
}
fn default_max_response() -> String {
    "10MB".to_string()
}
fn default_concurrency() -> u32 {
    4
}
fn default_retry_attempts() -> u32 {
    3
}
fn default_backoff() -> String {
    "1s".to_string()
}
fn default_true() -> bool {
    true
}

/// Execution context for operations
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub run_id: Uuid,
    pub sandbox_root: String,
    pub allowed_domains: Vec<String>,
    pub variables: HashMap<String, String>,
}

/// Result of operation execution with Meta² bits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub output: String,
    pub bits: Bits,
    pub duration_ms: u64,
    pub metadata: HashMap<String, String>,
}

/// Meta² Bits system - reflexive memory (A,U,P,E,Δ,I,R,T)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bits {
    /// Alignment - task aligns with goal
    #[serde(rename = "A")]
    pub alignment: u8,
    /// Uncertainty - confidence in result
    #[serde(rename = "U")]
    pub uncertainty: u8,
    /// Permission - human approval needed
    #[serde(rename = "P")]
    pub permission: u8,
    /// Error - something went wrong
    #[serde(rename = "E")]
    pub error: u8,
    /// Delta - context changed, need refresh
    #[serde(rename = "Δ")]
    pub delta: u8,
    /// Interrupt - external signal received
    #[serde(rename = "I")]
    pub interrupt: u8,
    /// Recovery - recovering from error
    #[serde(rename = "R")]
    pub recovery: u8,
    /// Trust - output can be trusted
    #[serde(rename = "T")]
    pub trust: u8,
}

impl Default for Bits {
    fn default() -> Self {
        Self {
            alignment: 1,
            uncertainty: 0,
            permission: 0,
            error: 0,
            delta: 0,
            interrupt: 0,
            recovery: 0,
            trust: 1,
        }
    }
}

impl Bits {
    /// Generate status line following Meta² convention (≤3 clauses)
    /// Priority: E > P > Δ > U > I > R > A > T
    pub fn status_line(&self) -> String {
        let mut clauses = Vec::new();

        if self.error == 1 {
            clauses.push("Error detected".to_string());
        }
        if self.permission == 1 {
            clauses.push("Awaiting approval".to_string());
        }
        if self.delta == 1 {
            clauses.push("Context stale".to_string());
        }
        if self.uncertainty == 1 {
            clauses.push("Uncertain result".to_string());
        }
        if self.interrupt == 1 {
            clauses.push("Interrupted".to_string());
        }
        if self.recovery == 1 {
            clauses.push("Recovering".to_string());
        }
        if self.alignment == 0 {
            clauses.push("Misaligned".to_string());
        }
        if self.trust == 0 {
            clauses.push("Low trust".to_string());
        }

        if clauses.is_empty() {
            "Ready".to_string()
        } else {
            clauses.into_iter().take(3).collect::<Vec<_>>().join(", ")
        }
    }

    /// Check if operation should proceed (Ask/Act gate)
    pub fn can_act(&self) -> bool {
        self.alignment == 1 && self.permission == 1 && self.delta == 0
    }
}

impl Operation {
    /// Return a concise label for the operation kind
    pub fn kind(&self) -> &'static str {
        match self {
            Operation::Shell { .. } => "shell",
            Operation::FsRead { .. } => "fs.read",
            Operation::FsWrite { .. } => "fs.write",
            Operation::HttpGet { .. } => "http.get",
            Operation::GitPatch { .. } => "git.patch",
            Operation::AssertFileExists { .. } => "assert.file_exists",
            Operation::AssertShellSuccess { .. } => "assert.shell_success",
            Operation::Sequence { .. } => "sequence",
            Operation::Parallel { .. } => "parallel",
            Operation::Conditional { .. } => "conditional",
            Operation::Retry { .. } => "retry",
        }
    }

    /// Provide a human-readable descriptor for logging/receipts
    pub fn descriptor(&self) -> String {
        match self {
            Operation::Shell { command, .. } => command.clone(),
            Operation::FsRead { path, .. } => format!("read: {}", path),
            Operation::FsWrite { path, .. } => format!("write: {}", path),
            Operation::HttpGet { url, .. } => format!("GET {}", url),
            Operation::GitPatch { repo_path, .. } => format!("git patch -> {}", repo_path),
            Operation::AssertFileExists { path } => format!("assert exists: {}", path),
            Operation::AssertShellSuccess { command, .. } => {
                format!("assert shell success: {}", command)
            }
            Operation::Sequence { steps } => format!("sequence ({} steps)", steps.len()),
            Operation::Parallel { steps, .. } => format!("parallel ({} steps)", steps.len()),
            Operation::Conditional { .. } => "conditional".to_string(),
            Operation::Retry { .. } => "retry".to_string(),
        }
    }
}

/// Parse UTIR document from YAML
pub fn parse_utir(content: &str) -> anyhow::Result<UtirDocument> {
    serde_yaml::from_str(content).map_err(Into::into)
}

/// Parse UTIR document from JSON
pub fn parse_utir_json(content: &str) -> anyhow::Result<UtirDocument> {
    serde_json::from_str(content).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits_status_line() {
        let mut bits = Bits::default();
        assert_eq!(bits.status_line(), "Ready");

        bits.error = 1;
        assert_eq!(bits.status_line(), "Error detected");

        bits.permission = 1;
        bits.uncertainty = 1;
        bits.interrupt = 1;
        assert_eq!(
            bits.status_line(),
            "Error detected, Awaiting approval, Uncertain result"
        );
    }

    #[test]
    fn test_can_act_gate() {
        let mut bits = Bits::default();
        bits.permission = 1;
        assert!(bits.can_act());

        bits.delta = 1;
        assert!(!bits.can_act());
    }

    #[test]
    fn test_parse_simple_utir() {
        let yaml = r#"
task_id: "test-task"
description: "Test task"
operations:
  - type: "shell"
    command: "echo hello"
  - type: "assert.shell_success"
    command: "echo world"
"#;

        let doc = parse_utir(yaml).unwrap();
        assert_eq!(doc.task_id, "test-task");
        assert_eq!(doc.operations.len(), 2);
    }
}
