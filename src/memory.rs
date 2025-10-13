use crate::utir::{Bits, OperationResult, UtirDocument};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use uuid::Uuid;

/// The Innate Memory System - crystallizes successful execution patterns into DNA
#[derive(Debug, Clone)]
pub struct MemorySystem {
    pub ledger_path: PathBuf,
    pub ghost_threshold: f64, // Success rate to crystallize patterns
    pub pattern_db: PatternDatabase,
}

/// A "ghost" - ephemeral memory of a successful execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGhost {
    pub run_id: Uuid,
    pub task_id: String,
    pub goal_description: String,
    pub utir_pattern: String, // Hash of the UTIR structure
    pub success_rate: f64,
    pub execution_time_ms: u64,
    pub bits_history: Vec<Bits>,
    pub pattern_signature: String,
    pub timestamp: u64,
    pub crystallization_score: f64,
}

/// Crystallized patterns that become part of the engine's DNA  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedPattern {
    pub pattern_id: Uuid,
    pub name: String,
    pub description: String,
    pub utir_template: String,
    pub success_metrics: SuccessMetrics,
    pub innate_constraints: Vec<String>,
    pub verification_rules: Vec<String>,
    pub crystallization_date: u64,
    pub usage_count: u64,
    pub evolution_history: Vec<PatternEvolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessMetrics {
    pub avg_success_rate: f64,
    pub avg_execution_time_ms: u64,
    pub error_patterns: Vec<String>,
    pub trust_distribution: Vec<f64>,
    pub bits_patterns: BitsPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitsPatterns {
    /// Reflexive patterns - automatic responses to states
    pub alignment_triggers: Vec<String>, // What causes A to flip
    pub uncertainty_patterns: Vec<String>, // Common U=1 scenarios
    pub permission_gates: Vec<String>,     // When P approval needed
    pub error_signatures: Vec<String>,     // E bit pattern matching
    pub context_invalidators: Vec<String>, // Δ reset conditions
    pub interrupt_handlers: Vec<String>,   // I bit recovery
    pub recovery_strategies: Vec<String>,  // R bit patterns
    pub trust_builders: Vec<String>,       // T reinforcement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolution {
    pub timestamp: u64,
    pub change_type: String, // "optimization", "constraint_added", "verification_improved"
    pub description: String,
    pub success_delta: f64,
}

/// Pattern database - the engine's crystallized intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDatabase {
    pub crystallized_patterns: HashMap<String, CrystallizedPattern>,
    pub active_ghosts: HashMap<Uuid, ExecutionGhost>,
    pub pattern_relationships: HashMap<String, Vec<String>>, // Pattern dependencies
    pub evolution_log: Vec<PatternEvolution>,
    pub innate_reflexes: BitsPatterns,
}

impl Default for PatternDatabase {
    fn default() -> Self {
        Self {
            crystallized_patterns: HashMap::new(),
            active_ghosts: HashMap::new(),
            pattern_relationships: HashMap::new(),
            evolution_log: Vec::new(),
            innate_reflexes: BitsPatterns {
                alignment_triggers: vec![
                    "goal_mismatch_detected".to_string(),
                    "output_quality_low".to_string(),
                ],
                uncertainty_patterns: vec![
                    "ambiguous_command".to_string(),
                    "novel_pattern_encountered".to_string(),
                    "verification_inconclusive".to_string(),
                ],
                permission_gates: vec![
                    "destructive_file_operation".to_string(),
                    "network_request_external".to_string(),
                    "system_level_command".to_string(),
                ],
                error_signatures: vec![
                    "command_not_found".to_string(),
                    "permission_denied".to_string(),
                    "timeout_exceeded".to_string(),
                ],
                context_invalidators: vec![
                    "file_system_changed".to_string(),
                    "external_dependency_updated".to_string(),
                ],
                interrupt_handlers: vec![
                    "user_abort_signal".to_string(),
                    "resource_exhaustion".to_string(),
                ],
                recovery_strategies: vec![
                    "retry_with_backoff".to_string(),
                    "fallback_to_safe_mode".to_string(),
                ],
                trust_builders: vec![
                    "verification_passed".to_string(),
                    "expected_outcome_achieved".to_string(),
                ],
            },
        }
    }
}

impl MemorySystem {
    pub fn new(ledger_path: PathBuf) -> Self {
        Self {
            ledger_path,
            ghost_threshold: 0.85, // 85% success rate to consider crystallization
            pattern_db: PatternDatabase::default(),
        }
    }

    /// Record a new execution as a ghost
    pub async fn record_execution(
        &mut self,
        doc: &UtirDocument,
        results: &[OperationResult],
    ) -> Result<ExecutionGhost> {
        let run_id = Uuid::new_v4();
        let success_rate = results
            .iter()
            .map(|r| if r.success { 1.0 } else { 0.0 })
            .sum::<f64>()
            / results.len() as f64;
        let total_time = results.iter().map(|r| r.duration_ms).sum();

        let ghost = ExecutionGhost {
            run_id,
            task_id: doc.task_id.clone(),
            goal_description: doc.description.clone(),
            utir_pattern: self.compute_pattern_hash(doc),
            success_rate,
            execution_time_ms: total_time,
            bits_history: results.iter().map(|r| r.bits.clone()).collect(),
            pattern_signature: self.compute_pattern_signature(doc, results),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            crystallization_score: self.compute_crystallization_score(
                success_rate,
                total_time,
                results,
            ),
        };

        // Store in active ghosts for analysis
        self.pattern_db.active_ghosts.insert(run_id, ghost.clone());

        // Write to append-only ledger (the geological record)
        self.append_to_ledger(&ghost).await?;

        // Check if this ghost should be crystallized
        if ghost.crystallization_score > self.ghost_threshold {
            self.crystallize_ghost(&ghost).await?;
        }

        Ok(ghost)
    }

    /// Crystallize a ghost into permanent engine DNA
    pub async fn crystallize_ghost(
        &mut self,
        ghost: &ExecutionGhost,
    ) -> Result<CrystallizedPattern> {
        let pattern_id = Uuid::new_v4();
        let pattern_name = format!("crystallized_{}", ghost.pattern_signature);

        let crystallized = CrystallizedPattern {
            pattern_id,
            name: pattern_name.clone(),
            description: format!(
                "Auto-crystallized from successful execution: {}",
                ghost.goal_description
            ),
            utir_template: ghost.utir_pattern.clone(),
            success_metrics: SuccessMetrics {
                avg_success_rate: ghost.success_rate,
                avg_execution_time_ms: ghost.execution_time_ms,
                error_patterns: self.extract_error_patterns(&ghost.bits_history),
                trust_distribution: ghost.bits_history.iter().map(|b| b.trust as f64).collect(),
                bits_patterns: self.analyze_bits_patterns(&ghost.bits_history),
            },
            innate_constraints: self.extract_safety_constraints(ghost),
            verification_rules: self.extract_verification_rules(ghost),
            crystallization_date: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            usage_count: 1,
            evolution_history: vec![PatternEvolution {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                change_type: "initial_crystallization".to_string(),
                description: "Pattern crystallized from successful ghost".to_string(),
                success_delta: ghost.success_rate,
            }],
        };

        // Store in pattern database - this becomes part of the engine's DNA
        self.pattern_db
            .crystallized_patterns
            .insert(pattern_name, crystallized.clone());

        // Update innate reflexes based on this pattern
        self.evolve_innate_reflexes(&crystallized);

        // Save pattern database
        self.save_pattern_database().await?;

        tracing::info!(
            "Crystallized new pattern: {} (score: {:.3})",
            crystallized.name,
            ghost.crystallization_score
        );

        Ok(crystallized)
    }

    /// Update the engine's innate reflexes based on crystallized patterns
    fn evolve_innate_reflexes(&mut self, pattern: &CrystallizedPattern) {
        // Analyze the pattern's bits patterns and integrate into innate reflexes
        let bits_patterns = &pattern.success_metrics.bits_patterns;

        // Merge new alignment triggers
        for trigger in &bits_patterns.alignment_triggers {
            if !self
                .pattern_db
                .innate_reflexes
                .alignment_triggers
                .contains(trigger)
            {
                self.pattern_db
                    .innate_reflexes
                    .alignment_triggers
                    .push(trigger.clone());
            }
        }

        // Merge other patterns similarly
        for pattern_type in &bits_patterns.uncertainty_patterns {
            if !self
                .pattern_db
                .innate_reflexes
                .uncertainty_patterns
                .contains(pattern_type)
            {
                self.pattern_db
                    .innate_reflexes
                    .uncertainty_patterns
                    .push(pattern_type.clone());
            }
        }

        // This is where the ghost becomes DNA - the successful patterns become
        // involuntary reflexes that fire automatically in future executions
    }

    /// Compute reflexive bits based on current execution state
    pub fn compute_reflexive_bits(&self, operation: &str, output: &str, success: bool) -> Bits {
        let mut bits = Bits::default();

        // Apply innate reflexes - these fire automatically based on crystallized patterns

        // Error detection (E bit)
        if !success
            || self
                .pattern_db
                .innate_reflexes
                .error_signatures
                .iter()
                .any(|sig| output.contains(sig))
        {
            bits.error = 1;
            bits.trust = 0; // Low trust on errors
        }

        // Permission gate (P bit)
        if self
            .pattern_db
            .innate_reflexes
            .permission_gates
            .iter()
            .any(|gate| operation.contains(gate))
        {
            bits.permission = 1; // Requires approval
        }

        // Uncertainty (U bit)
        if self
            .pattern_db
            .innate_reflexes
            .uncertainty_patterns
            .iter()
            .any(|pattern| output.contains(pattern))
        {
            bits.uncertainty = 1;
        }

        // Context change (Δ bit)
        if self
            .pattern_db
            .innate_reflexes
            .context_invalidators
            .iter()
            .any(|inv| output.contains(inv))
        {
            bits.delta = 1;
        }

        // Trust building (T bit)
        if success
            && self
                .pattern_db
                .innate_reflexes
                .trust_builders
                .iter()
                .any(|builder| output.contains(builder))
        {
            bits.trust = 1;
        }

        bits
    }

    /// Load existing patterns from disk - the engine's persistent memory
    pub async fn load_pattern_database(&mut self) -> Result<()> {
        let pattern_file = self.ledger_path.join("patterns.json");

        if pattern_file.exists() {
            let content = fs::read_to_string(&pattern_file).await?;
            self.pattern_db = serde_json::from_str(&content)?;
        }

        Ok(())
    }

    /// Save pattern database to disk
    async fn save_pattern_database(&self) -> Result<()> {
        let pattern_file = self.ledger_path.join("patterns.json");

        if let Some(parent) = pattern_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&self.pattern_db)?;
        fs::write(&pattern_file, content).await?;

        Ok(())
    }

    /// Append ghost to the geological ledger
    async fn append_to_ledger(&self, ghost: &ExecutionGhost) -> Result<()> {
        let ledger_file = self.ledger_path.join("ledger.jsonl");

        if let Some(parent) = ledger_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json_line = serde_json::to_string(ghost)? + "\n";

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ledger_file)
            .await?;

        file.write_all(json_line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    // Pattern analysis methods
    fn compute_pattern_hash(&self, doc: &UtirDocument) -> String {
        use sha2::{Digest, Sha256};

        let pattern_str = format!("{:?}", doc.operations);
        let mut hasher = Sha256::new();
        hasher.update(pattern_str.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    fn compute_pattern_signature(&self, doc: &UtirDocument, results: &[OperationResult]) -> String {
        let op_types: Vec<String> = doc
            .operations
            .iter()
            .map(|op| match op {
                crate::utir::Operation::Shell { .. } => "shell",
                crate::utir::Operation::FsRead { .. } => "fs_read",
                crate::utir::Operation::FsWrite { .. } => "fs_write",
                crate::utir::Operation::HttpGet { .. } => "http_get",
                crate::utir::Operation::GitPatch { .. } => "git_patch",
                crate::utir::Operation::AssertFileExists { .. } => "assert_file",
                crate::utir::Operation::AssertShellSuccess { .. } => "assert_shell",
                crate::utir::Operation::Sequence { .. } => "sequence",
                crate::utir::Operation::Parallel { .. } => "parallel",
                crate::utir::Operation::Conditional { .. } => "conditional",
                crate::utir::Operation::Retry { .. } => "retry",
            })
            .map(|s| s.to_string())
            .collect();

        let success_pattern = if results.iter().all(|r| r.success) {
            "all_success"
        } else {
            "mixed_results"
        };

        format!("{}_{}", op_types.join("_"), success_pattern)
    }

    fn compute_crystallization_score(
        &self,
        success_rate: f64,
        execution_time: u64,
        results: &[OperationResult],
    ) -> f64 {
        // Higher success rate = higher score
        let success_component = success_rate;

        // Lower execution time = higher score (with reasonable bounds)
        let time_component = 1.0 / (1.0 + (execution_time as f64 / 1000.0).log10());

        // High trust scores = higher score
        let trust_component =
            results.iter().map(|r| r.bits.trust as f64).sum::<f64>() / results.len() as f64;

        // Low error/uncertainty = higher score
        let stability_component = 1.0
            - (results
                .iter()
                .map(|r| (r.bits.error + r.bits.uncertainty) as f64)
                .sum::<f64>()
                / (results.len() as f64 * 2.0));

        success_component * 0.4
            + time_component * 0.2
            + trust_component * 0.2
            + stability_component * 0.2
    }

    fn extract_error_patterns(&self, bits_history: &[Bits]) -> Vec<String> {
        bits_history
            .iter()
            .filter(|b| b.error == 1)
            .enumerate()
            .map(|(i, _)| format!("error_at_step_{}", i))
            .collect()
    }

    fn analyze_bits_patterns(&self, bits_history: &[Bits]) -> BitsPatterns {
        BitsPatterns {
            alignment_triggers: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.alignment == 0)
                .map(|(i, _)| format!("misalignment_step_{}", i))
                .collect(),
            uncertainty_patterns: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.uncertainty == 1)
                .map(|(i, _)| format!("uncertainty_step_{}", i))
                .collect(),
            permission_gates: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.permission == 1)
                .map(|(i, _)| format!("permission_step_{}", i))
                .collect(),
            error_signatures: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.error == 1)
                .map(|(i, _)| format!("error_step_{}", i))
                .collect(),
            context_invalidators: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.delta == 1)
                .map(|(i, _)| format!("context_change_step_{}", i))
                .collect(),
            interrupt_handlers: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.interrupt == 1)
                .map(|(i, _)| format!("interrupt_step_{}", i))
                .collect(),
            recovery_strategies: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.recovery == 1)
                .map(|(i, _)| format!("recovery_step_{}", i))
                .collect(),
            trust_builders: bits_history
                .iter()
                .enumerate()
                .filter(|(_, b)| b.trust == 1)
                .map(|(i, _)| format!("trust_step_{}", i))
                .collect(),
        }
    }

    fn extract_safety_constraints(&self, ghost: &ExecutionGhost) -> Vec<String> {
        // Analyze successful patterns to extract safety constraints
        let mut constraints = Vec::new();

        if ghost.bits_history.iter().all(|b| b.permission == 1) {
            constraints.push("requires_permission_gate".to_string());
        }

        if ghost.success_rate > 0.9 {
            constraints.push("high_confidence_pattern".to_string());
        }

        constraints
    }

    fn extract_verification_rules(&self, ghost: &ExecutionGhost) -> Vec<String> {
        // Extract verification patterns that led to success
        let mut rules = Vec::new();

        if ghost.bits_history.iter().all(|b| b.trust == 1) {
            rules.push("verify_all_operations_trusted".to_string());
        }

        if ghost.bits_history.iter().all(|b| b.error == 0) {
            rules.push("no_errors_tolerated".to_string());
        }

        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ghost_crystallization() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = MemorySystem::new(temp_dir.path().to_path_buf());

        let doc = UtirDocument {
            task_id: "test".to_string(),
            description: "Test task".to_string(),
            operations: vec![],
            policy: None,
            bits_tracking: None,
        };

        let results = vec![OperationResult {
            success: true,
            output: "Success".to_string(),
            bits: Bits::default(),
            duration_ms: 100,
            metadata: HashMap::new(),
        }];

        let ghost = memory.record_execution(&doc, &results).await.unwrap();
        assert!(memory.pattern_db.active_ghosts.contains_key(&ghost.run_id));
    }

    #[tokio::test]
    async fn test_crystallization_updates_reflexes_and_persists() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = MemorySystem::new(temp_dir.path().to_path_buf());

        let doc = UtirDocument {
            task_id: "crystallize".to_string(),
            description: "Validate crystallization and reflex evolution".to_string(),
            operations: vec![
                crate::utir::Operation::Shell {
                    command: "echo success".to_string(),
                    timeout: "30s".to_string(),
                    working_dir: None,
                    env: HashMap::new(),
                    allow_network: false,
                    capture_output: true,
                },
                crate::utir::Operation::AssertShellSuccess {
                    command: "echo verify".to_string(),
                    timeout: "30s".to_string(),
                    expected_output: None,
                },
            ],
            policy: None,
            bits_tracking: None,
        };

        let mut first_bits = Bits::default();
        first_bits.permission = 1;

        let mut second_bits = Bits::default();
        second_bits.permission = 1;
        second_bits.alignment = 0;
        second_bits.uncertainty = 1;

        let results = vec![
            OperationResult {
                success: true,
                output: "expected_outcome_achieved".to_string(),
                bits: first_bits,
                duration_ms: 1_200,
                metadata: HashMap::new(),
            },
            OperationResult {
                success: true,
                output: "verification_passed".to_string(),
                bits: second_bits,
                duration_ms: 800,
                metadata: HashMap::new(),
            },
        ];

        let ghost = memory.record_execution(&doc, &results).await.unwrap();
        let pattern_name = format!("crystallized_{}", ghost.pattern_signature);

        assert!(ghost.crystallization_score > memory.ghost_threshold);
        assert!(memory
            .pattern_db
            .crystallized_patterns
            .contains_key(&pattern_name));

        let ledger_path = temp_dir.path().join("ledger.jsonl");
        assert!(tokio::fs::metadata(&ledger_path).await.is_ok());

        let patterns_path = temp_dir.path().join("patterns.json");
        let persisted = tokio::fs::read_to_string(&patterns_path)
            .await
            .expect("patterns.json should exist");
        let persisted_db: PatternDatabase = serde_json::from_str(&persisted).unwrap();
        assert!(persisted_db.crystallized_patterns.contains_key(&pattern_name));

        let alignment_trigger = "misalignment_step_1".to_string();
        let uncertainty_pattern = "uncertainty_step_1".to_string();
        assert!(memory
            .pattern_db
            .innate_reflexes
            .alignment_triggers
            .contains(&alignment_trigger));
        assert!(memory
            .pattern_db
            .innate_reflexes
            .uncertainty_patterns
            .contains(&uncertainty_pattern));

        let mut rehydrated = MemorySystem::new(temp_dir.path().to_path_buf());
        rehydrated
            .load_pattern_database()
            .await
            .expect("pattern database should load");
        assert!(rehydrated
            .pattern_db
            .crystallized_patterns
            .contains_key(&pattern_name));

        let permission_bits = rehydrated.compute_reflexive_bits(
            "destructive_file_operation",
            "expected_outcome_achieved",
            true,
        );
        assert_eq!(permission_bits.permission, 1);

        let mut gate_bits = Bits::default();
        gate_bits.permission = 1;
        assert!(gate_bits.can_act());
        gate_bits.delta = 1;
        assert!(!gate_bits.can_act());
    }

    #[test]
    fn test_reflexive_bits_computation() {
        let memory = MemorySystem::new(PathBuf::from("/tmp"));
        let bits = memory.compute_reflexive_bits("rm -rf /", "permission_denied", false);

        assert_eq!(bits.error, 1); // Error detected
        assert_eq!(bits.trust, 0); // Low trust
    }
}
