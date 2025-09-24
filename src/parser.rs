use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed representation of a chat instruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParsedInstruction {
    CreateApi(CreateApiSpec),
    CallApi(CallApiSpec),
    ApprovePattern { name: String },
    Unknown { original: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateApiSpec {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ApiParameterSpec>,
    pub return_description: Option<String>,
    pub persistence: PersistenceDirective,
    pub behavioral_hint: BehavioralHint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiParameterSpec {
    pub name: String,
    pub param_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallApiSpec {
    pub name: String,
    pub arguments: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistenceDirective {
    Ephemeral,
    Persist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BehavioralHint {
    Echo,
    PassThrough,
    Custom(String),
}

/// Primary entry point for parsing natural instructions into structured intents.
pub fn parse_instruction(input: &str) -> ParsedInstruction {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return ParsedInstruction::Unknown {
            original: input.to_string(),
        };
    }

    let normalized = trimmed.to_lowercase();
    if normalized.starts_with("define") || normalized.starts_with("create") {
        return parse_create_api(trimmed);
    }

    if normalized.starts_with("call") || normalized.starts_with("invoke") {
        return parse_call_api(trimmed);
    }

    if normalized.contains("approve") && normalized.contains("pattern") {
        if let Some(name) = extract_quoted_identifier(trimmed) {
            return ParsedInstruction::ApprovePattern { name };
        }
    }

    ParsedInstruction::Unknown {
        original: input.to_string(),
    }
}

fn parse_create_api(input: &str) -> ParsedInstruction {
    let name = extract_named_entity(input).unwrap_or_else(|| "unnamed".to_string());
    let params = extract_parameters(input);
    let persistence = if input.to_lowercase().contains("persistent")
        || input.to_lowercase().contains("permanent")
        || input.to_lowercase().contains("remember")
    {
        PersistenceDirective::Persist
    } else {
        PersistenceDirective::Ephemeral
    };

    let behavioral_hint = if input.to_lowercase().contains("returns it")
        && input.to_lowercase().contains("unmodified")
    {
        BehavioralHint::Echo
    } else if input.to_lowercase().contains("pass through") {
        BehavioralHint::PassThrough
    } else {
        BehavioralHint::Custom(input.to_string())
    };

    ParsedInstruction::CreateApi(CreateApiSpec {
        name,
        description: input.to_string(),
        parameters: params,
        return_description: extract_return_description(input),
        persistence,
        behavioral_hint,
    })
}

fn parse_call_api(input: &str) -> ParsedInstruction {
    let name = extract_named_entity(input).unwrap_or_else(|| "unnamed".to_string());
    let mut arguments = HashMap::new();

    // Look for simple pattern: with key='value'
    let arg_regex = Regex::new(r"(\w+)\s*=\s*'([^']*)'").unwrap();
    for caps in arg_regex.captures_iter(input) {
        if let (Some(key), Some(value)) = (caps.get(1), caps.get(2)) {
            arguments.insert(key.as_str().to_string(), value.as_str().to_string());
        }
    }

    ParsedInstruction::CallApi(CallApiSpec { name, arguments })
}

fn extract_named_entity(input: &str) -> Option<String> {
    extract_quoted_identifier(input).or_else(|| {
        let name_regex = Regex::new(r"named\s+([A-Za-z0-9_\-]+)").unwrap();
        name_regex
            .captures(input)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    })
}

fn extract_quoted_identifier(input: &str) -> Option<String> {
    let double_regex = Regex::new("\"([A-Za-z0-9_\\-]+)\"").unwrap();
    if let Some(caps) = double_regex.captures(input) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }

    let single_regex = Regex::new("'([A-Za-z0-9_\\-]+)'").unwrap();
    single_regex
        .captures(input)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn extract_parameters(input: &str) -> Vec<ApiParameterSpec> {
    let mut params = Vec::new();

    let param_regex = Regex::new("parameter[s]?[^'\"]*['\"]([A-Za-z0-9_]+)['\"]").unwrap();
    for caps in param_regex.captures_iter(input) {
        if let Some(name_match) = caps.get(1) {
            params.push(ApiParameterSpec {
                name: name_match.as_str().to_string(),
                param_type: None,
                description: Some("parameter inferred from instruction".to_string()),
            });
        }
    }

    if params.is_empty() {
        let fallback_regex = Regex::new("accepts\\s+(?:a|an)\\s+([A-Za-z0-9_]+)").unwrap();
        if let Some(caps) = fallback_regex.captures(&input.to_lowercase()) {
            if let Some(name_match) = caps.get(1) {
                params.push(ApiParameterSpec {
                    name: name_match.as_str().to_string(),
                    param_type: None,
                    description: None,
                });
            }
        }
    }

    params
}

fn extract_return_description(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    if lower.contains("returns") {
        let parts: Vec<&str> = lower.split("returns").collect();
        if let Some(part) = parts.get(1) {
            return Some(part.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_echo_definition() {
        let input = "Define a simple API named 'echo' that accepts a single parameter 'text' and returns it unmodified.";
        let parsed = parse_instruction(input);
        match parsed {
            ParsedInstruction::CreateApi(spec) => {
                assert_eq!(spec.name, "echo");
                assert!(!spec.parameters.is_empty());
                assert!(matches!(spec.behavioral_hint, BehavioralHint::Echo));
            }
            _ => panic!("Unexpected parse result: {:?}", parsed),
        }
    }

    #[test]
    fn parses_call_instruction() {
        let input = "Call the API 'echo' with text='Hello'";
        let parsed = parse_instruction(input);
        match parsed {
            ParsedInstruction::CallApi(spec) => {
                assert_eq!(spec.name, "echo");
                assert_eq!(spec.arguments.get("text"), Some(&"Hello".to_string()));
            }
            _ => panic!("Expected call spec"),
        }
    }
}
