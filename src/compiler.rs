use crate::utir::{Bits, ExecutionContext, Operation, OperationResult, UtirDocument};
use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

/// The UTIR Compiler - translates operations to safe, executable scripts
#[derive(Debug)]
pub struct UtirCompiler {
    pub sandbox_root: TempDir,
    pub allowed_domains: Vec<String>,
    pub security_rules: SecurityRules,
}

#[derive(Debug, Clone)]
pub struct SecurityRules {
    pub max_file_size: u64,
    pub max_execution_time: Duration,
    pub allowed_commands: Vec<String>,
    pub blocked_patterns: Vec<Regex>,
}

impl Default for SecurityRules {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,             // 100MB
            max_execution_time: Duration::from_secs(300), // 5 minutes
            allowed_commands: vec![
                "echo".to_string(),
                "ls".to_string(),
                "cat".to_string(),
                "mkdir".to_string(),
                "rm".to_string(),
                "cp".to_string(),
                "mv".to_string(),
                "git".to_string(),
                "curl".to_string(),
                "python3".to_string(),
                "node".to_string(),
                "npm".to_string(),
                "cargo".to_string(),
                "test".to_string(),
                "grep".to_string(),
                "find".to_string(),
            ],
            blocked_patterns: vec![
                Regex::new(r"rm\s+-rf\s+/").unwrap(),
                Regex::new(r"sudo").unwrap(),
                Regex::new(r"chmod\s+777").unwrap(),
                Regex::new(r">\s*/dev/").unwrap(),
            ],
        }
    }
}

impl UtirCompiler {
    pub fn new(allowed_domains: Vec<String>) -> Result<Self> {
        // Host mode: allow using host filesystem directly (less isolation, for operator use).
        let host_mode = std::env::var("ENGINE_HOST_MODE").ok().as_deref() == Some("1");
        let sandbox_root = if host_mode {
            // Use '/' as root placeholder; no temp dir constraint.
            TempDir::new_in("/")?
        } else {
            TempDir::new()?
        };
        Ok(Self {
            sandbox_root,
            allowed_domains,
            security_rules: SecurityRules::default(),
        })
    }

    /// Compile and execute a UTIR document
    pub async fn execute(&mut self, doc: &UtirDocument) -> Result<Vec<OperationResult>> {
        let context = ExecutionContext {
            run_id: Uuid::new_v4(),
            sandbox_root: self.sandbox_root.path().to_string_lossy().to_string(),
            allowed_domains: self.allowed_domains.clone(),
            variables: HashMap::new(),
        };

        let mut results = Vec::new();
        for operation in &doc.operations {
            let result = self.execute_operation(operation, &context).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute a single operation with safety constraints
    async fn execute_operation(
        &self,
        operation: &Operation,
        context: &ExecutionContext,
    ) -> Result<OperationResult> {
        let start = Instant::now();
        let mut bits = Bits::default();
        bits.permission = 1;

        // Check if we can proceed (Ask/Act gate)
        if !bits.can_act() {
            bits.permission = 1; // Need permission
            return Ok(OperationResult {
                success: false,
                output: "Operation blocked by Ask/Act gate".to_string(),
                bits,
                duration_ms: start.elapsed().as_millis() as u64,
                metadata: HashMap::new(),
            });
        }

        let (success, output) = match operation {
            Operation::Shell {
                command,
                timeout: timeout_str,
                working_dir,
                env,
                allow_network,
                capture_output,
            } => {
                self.execute_shell(
                    command,
                    timeout_str,
                    working_dir.as_deref(),
                    env,
                    *allow_network,
                    *capture_output,
                    context,
                )
                .await?
            }

            Operation::FsRead {
                path,
                encoding: _,
                max_size,
            } => self.execute_fs_read(path, max_size, context).await?,

            Operation::FsWrite {
                path,
                content,
                mode: _,
                create_dirs,
            } => {
                self.execute_fs_write(path, content, *create_dirs, context)
                    .await?
            }

            Operation::HttpGet {
                url,
                headers,
                timeout: timeout_str,
                max_response_size,
            } => {
                self.execute_http_get(url, headers, timeout_str, max_response_size, context)
                    .await?
            }

            Operation::GitPatch {
                repo_path,
                patch_content,
                commit_message,
                author,
            } => {
                self.execute_git_patch(repo_path, patch_content, commit_message, author, context)
                    .await?
            }

            Operation::AssertFileExists { path } => {
                self.execute_assert_file_exists(path, context).await?
            }

            Operation::AssertShellSuccess {
                command,
                timeout: timeout_str,
                expected_output,
            } => {
                self.execute_assert_shell_success(
                    command,
                    timeout_str,
                    expected_output.as_deref(),
                    context,
                )
                .await?
            }

            Operation::Sequence { steps } => self.execute_sequence(steps, context).await?,

            Operation::Parallel {
                steps,
                max_concurrency,
            } => {
                self.execute_parallel(steps, *max_concurrency, context)
                    .await?
            }

            Operation::Conditional {
                condition,
                then_op,
                else_op,
            } => {
                self.execute_conditional(condition, then_op, else_op.as_deref(), context)
                    .await?
            }

            Operation::Retry {
                operation,
                max_attempts,
                backoff,
            } => {
                self.execute_retry(operation, *max_attempts, backoff, context)
                    .await?
            }
        };

        // Update bits based on result
        if !success {
            bits.error = 1; // Error occurred
            bits.trust = 0; // Low trust
        }

        Ok(OperationResult {
            success,
            output,
            bits,
            duration_ms: start.elapsed().as_millis() as u64,
            metadata: HashMap::new(),
        })
    }

    async fn execute_shell(
        &self,
        command: &str,
        timeout_str: &str,
        working_dir: Option<&str>,
        env: &HashMap<String, String>,
        _allow_network: bool,
        capture_output: bool,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        // Security check - validate command against rules
        if !self.is_command_safe(command)? {
            return Ok((false, "Command blocked by security rules".to_string()));
        }

        let timeout_duration = self.parse_duration(timeout_str)?;
        let host_mode = std::env::var("ENGINE_HOST_MODE").ok().as_deref() == Some("1");
        let work_dir = if let Some(dir) = working_dir {
            let p = PathBuf::from(dir);
            if host_mode && p.is_absolute() {
                p
            } else {
                PathBuf::from(&context.sandbox_root).join(dir)
            }
        } else {
            PathBuf::from(&context.sandbox_root)
        };

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command).current_dir(&work_dir).envs(env);

        if capture_output {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        }

        let child = cmd.spawn()?;
        let result = timeout(timeout_duration, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else {
                    format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
                };
                Ok((output.status.success(), combined))
            }
            Ok(Err(e)) => Ok((false, format!("Process error: {}", e))),
            Err(_) => Ok((false, "Command timed out".to_string())),
        }
    }

    async fn execute_fs_read(
        &self,
        path: &str,
        max_size: &str,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        let host_mode = std::env::var("ENGINE_HOST_MODE").ok().as_deref() == Some("1");
        let candidate = PathBuf::from(path);
        let full_path = if host_mode && candidate.is_absolute() {
            candidate
        } else {
            PathBuf::from(&context.sandbox_root).join(path)
        };

        // Security check - ensure path is within sandbox (unless host_mode)
        if !host_mode && !full_path.starts_with(&context.sandbox_root) {
            return Ok((false, "Path outside sandbox".to_string()));
        }

        match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                let max_bytes = self.parse_size(max_size)?;
                if content.len() > max_bytes as usize {
                    Ok((false, "File too large".to_string()))
                } else {
                    Ok((true, content))
                }
            }
            Err(e) => Ok((false, format!("Failed to read file: {}", e))),
        }
    }

    async fn execute_fs_write(
        &self,
        path: &str,
        content: &str,
        create_dirs: bool,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        let host_mode = std::env::var("ENGINE_HOST_MODE").ok().as_deref() == Some("1");
        let candidate = PathBuf::from(path);
        let full_path = if host_mode && candidate.is_absolute() {
            candidate
        } else {
            PathBuf::from(&context.sandbox_root).join(path)
        };

        // Security check - ensure path is within sandbox (unless host_mode)
        if !host_mode && !full_path.starts_with(&context.sandbox_root) {
            return Ok((false, "Path outside sandbox".to_string()));
        }

        if create_dirs {
            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        match tokio::fs::write(&full_path, content).await {
            Ok(()) => Ok((true, format!("Written {} bytes to {}", content.len(), path))),
            Err(e) => Ok((false, format!("Failed to write file: {}", e))),
        }
    }

    async fn execute_http_get(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        timeout_str: &str,
        max_response_size: &str,
        _context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        // Security check - validate domain
        if !self.is_url_allowed(url)? {
            return Ok((false, "URL not in allowed domains".to_string()));
        }

        let timeout_duration = self.parse_duration(timeout_str)?;
        let client = reqwest::Client::builder()
            .timeout(timeout_duration)
            .build()?;

        let mut request = client.get(url);
        for (key, value) in headers {
            request = request.header(key, value);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(text) => {
                            let max_bytes = self.parse_size(max_response_size)?;
                            if text.len() > max_bytes as usize {
                                Ok((false, "Response too large".to_string()))
                            } else {
                                Ok((true, text))
                            }
                        }
                        Err(e) => Ok((false, format!("Failed to read response: {}", e))),
                    }
                } else {
                    Ok((false, format!("HTTP error: {}", response.status())))
                }
            }
            Err(e) => Ok((false, format!("Request failed: {}", e))),
        }
    }

    async fn execute_git_patch(
        &self,
        repo_path: &str,
        patch_content: &str,
        commit_message: &str,
        author: &str,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        let full_path = PathBuf::from(&context.sandbox_root).join(repo_path);

        // Write patch to temporary file
        let patch_file = full_path.join("temp.patch");
        tokio::fs::write(&patch_file, patch_content).await?;

        // Apply patch
        let apply_result = Command::new("git")
            .args(&["apply", "--check", "temp.patch"])
            .current_dir(&full_path)
            .output()
            .await?;

        if !apply_result.status.success() {
            return Ok((false, "Patch does not apply cleanly".to_string()));
        }

        // Apply the patch
        let apply_result = Command::new("git")
            .args(&["apply", "temp.patch"])
            .current_dir(&full_path)
            .output()
            .await?;

        if !apply_result.status.success() {
            return Ok((false, "Failed to apply patch".to_string()));
        }

        // Commit changes
        let commit_result = Command::new("git")
            .args(&["commit", "-a", "-m", commit_message, "--author", author])
            .current_dir(&full_path)
            .output()
            .await?;

        let success = commit_result.status.success();
        let output = if success {
            "Patch applied and committed successfully".to_string()
        } else {
            format!(
                "Commit failed: {}",
                String::from_utf8_lossy(&commit_result.stderr)
            )
        };

        // Clean up
        let _ = tokio::fs::remove_file(patch_file).await;

        Ok((success, output))
    }

    async fn execute_assert_file_exists(
        &self,
        path: &str,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        let full_path = PathBuf::from(&context.sandbox_root).join(path);
        let exists = full_path.exists();

        Ok((
            exists,
            if exists {
                format!("File exists: {}", path)
            } else {
                format!("File does not exist: {}", path)
            },
        ))
    }

    async fn execute_assert_shell_success(
        &self,
        command: &str,
        timeout_str: &str,
        expected_output: Option<&str>,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        let (success, output) = self
            .execute_shell(
                command,
                timeout_str,
                None,
                &HashMap::new(),
                false,
                true,
                context,
            )
            .await?;

        if !success {
            return Ok((false, format!("Shell command failed: {}", output)));
        }

        if let Some(expected) = expected_output {
            if output.contains(expected) {
                Ok((
                    true,
                    "Shell command succeeded with expected output".to_string(),
                ))
            } else {
                Ok((
                    false,
                    format!("Output did not contain expected string: {}", expected),
                ))
            }
        } else {
            Ok((true, "Shell command succeeded".to_string()))
        }
    }

    fn execute_sequence<'a>(
        &'a self,
        steps: &'a [Operation],
        context: &'a ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(bool, String)>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut outputs = Vec::new();

            for step in steps {
                let result = Box::pin(self.execute_operation(step, context)).await?;
                outputs.push(format!("Step: {}", result.output));

                if !result.success {
                    return Ok((false, outputs.join("\n")));
                }
            }

            Ok((true, outputs.join("\n")))
        })
    }

    async fn execute_parallel(
        &self,
        steps: &[Operation],
        _max_concurrency: u32,
        context: &ExecutionContext,
    ) -> Result<(bool, String)> {
        // For now, execute sequentially (parallel execution would require more complex async handling)
        self.execute_sequence(steps, context).await
    }

    fn execute_conditional<'a>(
        &'a self,
        condition: &'a Operation,
        then_op: &'a Operation,
        else_op: Option<&'a Operation>,
        context: &'a ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(bool, String)>> + Send + 'a>>
    {
        Box::pin(async move {
            let condition_result = Box::pin(self.execute_operation(condition, context)).await?;

            if condition_result.success {
                let then_result = Box::pin(self.execute_operation(then_op, context)).await?;
                Ok((
                    then_result.success,
                    format!("Condition true, executed then: {}", then_result.output),
                ))
            } else if let Some(else_operation) = else_op {
                let else_result = Box::pin(self.execute_operation(else_operation, context)).await?;
                Ok((
                    else_result.success,
                    format!("Condition false, executed else: {}", else_result.output),
                ))
            } else {
                Ok((true, "Condition false, no else branch".to_string()))
            }
        })
    }

    fn execute_retry<'a>(
        &'a self,
        operation: &'a Operation,
        max_attempts: u32,
        backoff: &'a str,
        context: &'a ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(bool, String)>> + Send + 'a>>
    {
        Box::pin(async move {
            let backoff_duration = self.parse_duration(backoff)?;
            let mut last_error = String::new();

            for attempt in 1..=max_attempts {
                let result = Box::pin(self.execute_operation(operation, context)).await?;

                if result.success {
                    return Ok((
                        true,
                        format!("Succeeded on attempt {}: {}", attempt, result.output),
                    ));
                }

                last_error = result.output;

                if attempt < max_attempts {
                    tokio::time::sleep(backoff_duration).await;
                }
            }

            Ok((
                false,
                format!(
                    "Failed after {} attempts. Last error: {}",
                    max_attempts, last_error
                ),
            ))
        })
    }

    // Utility functions
    fn is_command_safe(&self, command: &str) -> Result<bool> {
        // Check against blocked patterns
        for pattern in &self.security_rules.blocked_patterns {
            if pattern.is_match(command) {
                return Ok(false);
            }
        }

        // Check if command starts with allowed commands
        let first_word = command.split_whitespace().next().unwrap_or("");
        Ok(self
            .security_rules
            .allowed_commands
            .iter()
            .any(|cmd| first_word == cmd))
    }

    fn is_url_allowed(&self, url: &str) -> Result<bool> {
        use url::Url;
        let parsed_url = Url::parse(url)?;

        if let Some(host) = parsed_url.host_str() {
            Ok(self
                .allowed_domains
                .iter()
                .any(|domain| host.ends_with(domain)))
        } else {
            Ok(false)
        }
    }

    fn parse_duration(&self, duration_str: &str) -> Result<Duration> {
        let re = Regex::new(r"(\d+)([smh])")?;

        if let Some(captures) = re.captures(duration_str) {
            let value: u64 = captures[1].parse()?;
            let unit = &captures[2];

            let duration = match unit {
                "s" => Duration::from_secs(value),
                "m" => Duration::from_secs(value * 60),
                "h" => Duration::from_secs(value * 3600),
                _ => return Err(anyhow!("Invalid duration unit")),
            };

            if duration > self.security_rules.max_execution_time {
                Ok(self.security_rules.max_execution_time)
            } else {
                Ok(duration)
            }
        } else {
            Err(anyhow!("Invalid duration format"))
        }
    }

    fn parse_size(&self, size_str: &str) -> Result<u64> {
        let re = Regex::new(r"(\d+)(MB|KB|GB)?")?;

        if let Some(captures) = re.captures(size_str) {
            let value: u64 = captures[1].parse()?;
            let unit = captures.get(2).map(|m| m.as_str()).unwrap_or("B");

            let size = match unit {
                "KB" => value * 1024,
                "MB" => value * 1024 * 1024,
                "GB" => value * 1024 * 1024 * 1024,
                "B" | "" => value,
                _ => return Err(anyhow!("Invalid size unit")),
            };

            if size > self.security_rules.max_file_size {
                Ok(self.security_rules.max_file_size)
            } else {
                Ok(size)
            }
        } else {
            Err(anyhow!("Invalid size format"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utir::parse_utir;

    #[tokio::test]
    async fn test_simple_shell_execution() {
        let mut compiler = UtirCompiler::new(vec!["api.example.com".to_string()]).unwrap();

        let utir_yaml = r#"
task_id: "test-echo"
description: "Simple echo test"
operations:
  - type: "shell"
    command: "echo hello world"
"#;

        let doc = parse_utir(utir_yaml).unwrap();
        let results = compiler.execute(&doc).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(results[0].output.contains("hello world"));
    }

    #[tokio::test]
    async fn test_security_blocked_command() {
        let mut compiler = UtirCompiler::new(vec![]).unwrap();

        let utir_yaml = r#"
task_id: "test-dangerous"
description: "Dangerous command test"  
operations:
  - type: "shell"
    command: "rm -rf /"
"#;

        let doc = parse_utir(utir_yaml).unwrap();
        let results = compiler.execute(&doc).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].output.contains("blocked by security"));
    }
}
