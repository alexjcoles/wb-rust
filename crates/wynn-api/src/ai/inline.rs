use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use wynn_core::db::ItemDb;

use super::provider::{AiError, AiProvider, AiRequest, AiResponse, ToolCall};
use super::tools::execute_tool;

/// Provider that shells out to `claude --print` for LLM reasoning.
/// Uses Claude Code CLI in non-interactive mode — no API keys needed,
/// uses the user's existing Claude Code auth.
///
/// Flow:
/// 1. Run tools locally (parse, analyze) to gather build context
/// 2. Pass context + user message to `claude --print`
/// 3. Get back structured analysis with suggestions
pub struct ClaudeCliProvider {
    db: Arc<ItemDb>,
    model: Option<String>,
}

impl ClaudeCliProvider {
    pub fn new(db: Arc<ItemDb>) -> Self {
        Self { db, model: None }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }
}

/// Structured response from claude --print --json-schema
#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponse {
    analysis: String,
    #[serde(default)]
    suggested_flex_slots: Vec<String>,
    #[serde(default)]
    suggested_objectives: Vec<String>,
}

/// Load the JSON schema from the schemas/ directory next to the binary,
/// or fall back to the one in the source tree.
fn load_schema() -> Result<String, AiError> {
    // Try relative to current working dir first (how the server runs)
    let candidates = [
        "crates/wynn-api/schemas/build-analysis.json",
        "schemas/build-analysis.json",
    ];
    for path in &candidates {
        if let Ok(schema) = std::fs::read_to_string(path) {
            return Ok(schema);
        }
    }
    Err(AiError::Provider(
        "could not find schemas/build-analysis.json".into(),
    ))
}

impl AiProvider for ClaudeCliProvider {
    fn chat<'a>(
        &'a self,
        request: AiRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<AiResponse, AiError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let user_msg = request
                .messages
                .iter()
                .rev()
                .find(|m| m.role == super::provider::Role::User)
                .ok_or_else(|| AiError::Provider("no user message".into()))?;

            // Extract URL and run tools locally for context
            let url = extract_url(&user_msg.content);
            let mut tool_calls = Vec::new();
            let mut context_parts = Vec::new();

            if let Some(ref url) = url {
                // Parse build
                let parse_call = ToolCall {
                    name: "parse_build".into(),
                    arguments: json!({ "url": url }),
                };
                let parse_result = execute_tool(&parse_call, &self.db);
                context_parts.push(format!(
                    "## Build Data (from parse_build)\n```json\n{}\n```",
                    serde_json::to_string_pretty(&parse_result.result).unwrap_or_default()
                ));
                tool_calls.push(parse_call);

                // Analyze build
                let analyze_call = ToolCall {
                    name: "analyze_build".into(),
                    arguments: json!({ "url": url }),
                };
                let analyze_result = execute_tool(&analyze_call, &self.db);
                context_parts.push(format!(
                    "## Build Analysis (from analyze_build)\n```json\n{}\n```",
                    serde_json::to_string_pretty(&analyze_result.result).unwrap_or_default()
                ));
                tool_calls.push(analyze_call);
            }

            // Build the prompt for claude --print
            let context = context_parts.join("\n\n");
            let prompt = format!(
                "{}\n\n---\n\n{}\n\n---\n\nUser message: {}",
                request.system_prompt, context, user_msg.content
            );

            // Call claude --print
            let claude_result = call_claude_cli(&prompt, self.model.as_deref()).await?;

            // If Claude suggested solver changes and we have a URL, run the solver
            let mut solver_text = String::new();
            let mut tool_results = Vec::new();
            if let Some(ref url) = url {
                if !claude_result.suggested_flex_slots.is_empty() {
                    let all_slots = [
                        "helmet", "chestplate", "leggings", "boots",
                        "ring1", "ring2", "bracelet", "necklace", "weapon",
                    ];
                    let locked: Vec<String> = all_slots
                        .iter()
                        .filter(|s| {
                            !claude_result
                                .suggested_flex_slots
                                .iter()
                                .any(|f| f.to_lowercase() == **s)
                        })
                        .map(|s| s.to_string())
                        .collect();

                    let solve_call = ToolCall {
                        name: "solve_build".into(),
                        arguments: json!({
                            "url": url,
                            "locked_slots": locked,
                            "objectives": claude_result.suggested_objectives,
                            "available_points": 250,
                            "max_results": 3,
                            "min_item_level": 90,
                        }),
                    };
                    let solve_result = execute_tool(&solve_call, &self.db);

                    if let Some(results) =
                        solve_result.result.get("results").and_then(|v| v.as_array())
                    {
                        if !results.is_empty() {
                            solver_text.push_str("\n\n### Solver Results\n");
                            for (i, r) in results.iter().enumerate() {
                                let items_changed: Vec<String> = r
                                    .get("items")
                                    .and_then(|v| v.as_array())
                                    .map(|items| {
                                        items
                                            .iter()
                                            .filter(|item| {
                                                let slot = item
                                                    .get("slot")
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("");
                                                claude_result.suggested_flex_slots.iter().any(
                                                    |f| f.eq_ignore_ascii_case(slot),
                                                )
                                            })
                                            .map(|item| {
                                                format!(
                                                    "{}: {}",
                                                    item.get("slot")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("?"),
                                                    item.get("name")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("?")
                                                )
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let url = r.get("url").and_then(|u| u.as_str()).unwrap_or("");
                                let hp = r.get("hp").and_then(|v| v.as_i64()).unwrap_or(0);
                                let ehp = r.get("ehp").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                solver_text.push_str(&format!(
                                    "{}. {} — HP: {}, EHP: {:.0} — [View build]({})\n",
                                    i + 1,
                                    items_changed.join(", "),
                                    hp,
                                    ehp,
                                    url,
                                ));
                            }
                        }
                    }

                    tool_calls.push(solve_call.clone());
                    tool_results.push(super::provider::ToolResult {
                        name: "solve_build".into(),
                        result: json!({}),
                    });
                }
            }

            let text = format!("{}{}", claude_result.analysis, solver_text);

            Ok(AiResponse {
                text,
                tool_calls_made: tool_calls,
                tool_results,
            })
        })
    }

    fn name(&self) -> &str {
        "claude-cli"
    }
}

/// Wrapper for the full claude --print JSON envelope
#[derive(Debug, Deserialize)]
struct ClaudeCliOutput {
    structured_output: Option<ClaudeResponse>,
    result: Option<String>,
}

async fn call_claude_cli(prompt: &str, model: Option<&str>) -> Result<ClaudeResponse, AiError> {
    let schema = load_schema()?;

    // Resolve claude binary — check common locations
    let claude_bin = resolve_claude_binary();

    let mut cmd = tokio::process::Command::new(&claude_bin);
    cmd.arg("--print")
        .arg("--output-format")
        .arg("json")
        .arg("--json-schema")
        .arg(&schema)
        .arg("--no-session-persistence")
        .arg(prompt);

    if let Some(model) = model {
        cmd.arg("--model").arg(model);
    }

    // Unset CLAUDECODE to avoid recursion issues
    cmd.env_remove("CLAUDECODE");

    let output = cmd
        .output()
        .await
        .map_err(|e| AiError::Provider(format!("failed to run claude CLI ({}): {e}", claude_bin)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AiError::Provider(format!(
            "claude CLI exited with {}: {}",
            output.status, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Claude --output-format json wraps everything in an envelope
    // with structured_output containing our schema'd response
    let envelope: ClaudeCliOutput = serde_json::from_str(&stdout).map_err(|e| {
        AiError::Provider(format!(
            "failed to parse claude CLI output: {e}\nraw: {}",
            &stdout[..stdout.len().min(500)]
        ))
    })?;

    envelope.structured_output.ok_or_else(|| {
        AiError::Provider(format!(
            "claude CLI returned no structured_output. result: {:?}",
            envelope.result
        ))
    })
}

fn resolve_claude_binary() -> String {
    // Try ~/.claude/local/claude first (where Claude Code installs)
    if let Some(home) = std::env::var_os("HOME") {
        let local = format!("{}/.claude/local/claude", home.to_string_lossy());
        if std::path::Path::new(&local).exists() {
            return local;
        }
    }
    // Fall back to PATH
    "claude".into()
}

fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != ':' && c != '/' && c != '#' && c != '+' && c != '-'
        });
        if word.contains("wynnbuilder.github.io/builder/#")
            || word.contains("hppeng-wynn.github.io/builder/#")
        {
            return Some(word.to_string());
        }
    }
    // Try bare hash
    for word in text.split_whitespace() {
        if word.len() > 20
            && (word.starts_with("CN")
                || word.starts_with("CK")
                || word.starts_with("CM")
                || word.starts_with("CL")
                || word.starts_with("CI")
                || word.starts_with("CE")
                || word.starts_with("CJ"))
        {
            return Some(word.to_string());
        }
    }
    None
}
