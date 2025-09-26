use anyhow::{Context, Result};
use one_engine::utir::{parse_utir, Operation};
use std::{env, fs};

fn main() {
    if let Err(err) = run() {
        eprintln!("utir_to_dot: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let input_path = args
        .next()
        .context("UTIR file path required (e.g. tasks/proof.utir.json)")?;
    let output_path = args.next();

    let raw = fs::read_to_string(&input_path)
        .with_context(|| format!("Failed to read UTIR file: {input_path}"))?;
    let utir_yaml = extract_utir_payload(&raw)?;
    let doc = parse_utir(&utir_yaml).context("Failed to parse UTIR payload")?;

    let mut dot = String::new();
    dot.push_str("digraph UTIR {\n  rankdir=LR;\n  node [shape=box, fontname=\"Helvetica\"];\n");

    for (idx, op) in doc.operations.iter().enumerate() {
        let label = format!(
            "{}: {}\\n{}",
            idx,
            op.type_name(),
            sanitize(&operation_detail(op))
        );
        dot.push_str(&format!("  n{idx} [label=\"{label}\"];\n"));
        if idx + 1 < doc.operations.len() {
            dot.push_str(&format!("  n{idx} -> n{};\n", idx + 1));
        }
    }

    dot.push_str("}\n");

    if let Some(path) = output_path {
        fs::write(&path, dot).with_context(|| format!("Failed to write DOT file: {path}"))?;
    } else {
        print!("{dot}");
    }

    Ok(())
}

fn extract_utir_payload(raw: &str) -> Result<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(payload) = json.get("utir").and_then(|v| v.as_str()) {
            return Ok(payload.to_string());
        }
    }
    Ok(raw.to_string())
}

fn operation_detail(operation: &Operation) -> String {
    match operation {
        Operation::Shell { command, .. } => truncate(command, 80),
        Operation::FsRead { path, .. } => format!("read {path}"),
        Operation::FsWrite { path, .. } => format!("write {path}"),
        Operation::HttpGet { url, .. } => truncate(url, 80),
        Operation::GitPatch { repo_path, .. } => format!("patch {}", truncate(repo_path, 48)),
        Operation::AssertFileExists { path } => format!("assert {path}"),
        Operation::AssertShellSuccess { command, .. } => truncate(command, 80),
        Operation::SummarizeRun { notes } => truncate(notes, 80),
        Operation::Sequence { steps } => format!("sequence steps: {}", steps.len()),
        Operation::Parallel { steps, .. } => format!("parallel steps: {}", steps.len()),
        Operation::Conditional { .. } => "conditional".to_string(),
        Operation::Retry { max_attempts, .. } => format!("retry x{max_attempts}"),
    }
}

fn sanitize(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn truncate(value: &str, max_len: usize) -> String {
    let mut result = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx >= max_len {
            break;
        }
        result.push(ch);
    }

    if value.chars().count() > max_len {
        result.push_str("...");
    }

    result
}
