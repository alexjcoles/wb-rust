use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Tool call requested by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub result: serde_json::Value,
}

/// Request to an AI provider.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub messages: Vec<Message>,
    pub system_prompt: String,
}

/// Response from an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    /// The assistant's text response to show the user.
    pub text: String,
    /// Tool calls the AI wants to make (provider handles the loop,
    /// so this is empty by the time we return to the user).
    pub tool_calls_made: Vec<ToolCall>,
    /// Results from tool calls, included for transparency.
    pub tool_results: Vec<ToolResult>,
}

/// Trait for AI providers. Each implementation handles its own
/// tool-use loop internally — the caller gets back a final response.
pub trait AiProvider: Send + Sync {
    fn chat<'a>(
        &'a self,
        request: AiRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AiResponse, AiError>> + Send + 'a>>;

    fn name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("configuration error: {0}")]
    Config(String),
}
