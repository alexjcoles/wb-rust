use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::ai::inline::ClaudeCliProvider;
use crate::ai::provider::{AiProvider, AiRequest, Message, Role};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    /// Provider to use: "claude-cli" (default), "anthropic", "openai", "ollama"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model override (e.g. "sonnet", "opus", "gpt-4o", "gemma3:27b")
    pub model: Option<String>,
}

fn default_provider() -> String {
    "claude-cli".into()
}

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub text: String,
    pub provider: String,
    pub tool_calls: Vec<serde_json::Value>,
}

const SYSTEM_PROMPT: &str = r#"You are a Wynncraft build advisor. Players paste WynnBuilder URLs and you help them understand and improve their builds.

You have been given the parsed build data and analysis results below. Use them to provide a clear, helpful response.

Key knowledge:
- Build archetypes: Spell builds need intelligence + mana regen (8/4s minimum). Melee/t-stack builds need attack speed + raw main attack damage.
- Survivability: EHP > raw HP. Life steal matters for lootruns. Walk speed below 0% is dangerous for dodge-heavy content.
- Element defences: Below -60 = critical (one-shot risk). Aim for 0+ on dangerous elements, ideally +30-50.
- SP budget: 200 points total (tomes removed in latest update). Items have minimum SP requirements that cascade.
- Hive rules: Max 1 item from each set group.

When suggesting changes:
- Set "suggested_flex_slots" to the slots you'd swap (e.g. ["boots", "ring1"])
- Set "suggested_objectives" to what the solver should maximise (e.g. ["ehp", "thunder_defence"])
- The solver will run automatically and append results

Be concise. Lead with the most important finding."#;

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "system" => Role::System,
                "assistant" => Role::Assistant,
                _ => Role::User,
            },
            content: m.content.clone(),
        })
        .collect();

    let ai_request = AiRequest {
        messages,
        system_prompt: SYSTEM_PROMPT.to_string(),
    };

    let provider_name = req.provider.as_str();

    match provider_name {
        "claude-cli" => {
            let mut provider = ClaudeCliProvider::new(state.db.clone());
            if let Some(model) = req.model {
                provider = provider.with_model(model);
            }

            let response = provider
                .chat(ai_request)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok(Json(ChatResponse {
                text: response.text,
                provider: "claude-cli".into(),
                tool_calls: response
                    .tool_calls_made
                    .iter()
                    .map(|tc| serde_json::json!({ "name": tc.name, "arguments": tc.arguments }))
                    .collect(),
            }))
        }
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("Provider '{other}' not yet implemented. Available: claude-cli"),
        )),
    }
}
