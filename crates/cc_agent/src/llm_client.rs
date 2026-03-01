use async_trait::async_trait;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Response from an LLM.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// A tool call the LLM wants to make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Error type for LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("API error: {0}")]
    Api(String),
}

/// Pluggable LLM client trait.
/// On native: requires Send + Sync for cross-thread use.
/// On WASM: single-threaded, no Send/Sync needed.
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError>;

    fn model_name(&self) -> &str;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait LlmClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError>;

    fn model_name(&self) -> &str;
}

/// Parse an OpenAI-compatible JSON response into LlmResponse.
/// Shared by OpenAiCompatibleClient and WebLlmClient.
pub fn parse_openai_response(json: &serde_json::Value) -> Result<LlmResponse, LlmError> {
    let choice = json["choices"]
        .get(0)
        .ok_or_else(|| LlmError::Api("No choices in response".into()))?;

    let content = choice["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let mut tool_calls = Vec::new();
    if let Some(calls) = choice["message"]["tool_calls"].as_array() {
        for call in calls {
            tool_calls.push(ToolCall {
                id: call["id"].as_str().unwrap_or("").to_string(),
                name: call["function"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                arguments: serde_json::from_str(
                    call["function"]["arguments"].as_str().unwrap_or("{}"),
                )
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            });
        }
    }

    Ok(LlmResponse {
        content,
        tool_calls,
    })
}

/// OpenAI-compatible client — works for Mistral API, vLLM, Ollama.
pub struct OpenAiCompatibleClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    client: reqwest::Client,
}

impl OpenAiCompatibleClient {
    pub fn new(base_url: String, api_key: String, model: String, temperature: f32) -> Self {
        Self {
            base_url,
            api_key,
            model,
            temperature,
            client: reqwest::Client::new(),
        }
    }

    async fn do_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": self.temperature,
        });

        if let Some(tools) = tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::Value::Array(tool_defs);
        }

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Http(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        parse_openai_response(&json)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        self.do_complete(messages, tools).await
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        self.do_complete(messages, tools).await
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Mock LLM client for testing — returns canned responses.
pub struct MockLlmClient {
    pub responses: Vec<LlmResponse>,
    response_idx: std::sync::atomic::AtomicUsize,
}

impl MockLlmClient {
    pub fn new(responses: Vec<LlmResponse>) -> Self {
        Self {
            responses,
            response_idx: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(
        &self,
        _messages: &[ChatMessage],
        _tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let idx = self
            .response_idx
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.responses
            .get(idx % self.responses.len())
            .cloned()
            .ok_or_else(|| LlmError::Api("No mock responses".into()))
    }

    fn model_name(&self) -> &str {
        "mock"
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl LlmClient for MockLlmClient {
    async fn complete(
        &self,
        _messages: &[ChatMessage],
        _tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let idx = self
            .response_idx
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.responses
            .get(idx % self.responses.len())
            .cloned()
            .ok_or_else(|| LlmError::Api("No mock responses".into()))
    }

    fn model_name(&self) -> &str {
        "mock"
    }
}

/// LLM configuration resource.
#[derive(Debug, Clone, Resource)]
pub struct LlmConfig {
    pub backend: LlmBackend,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    /// When true, the model is a fine-tuned Lua generator that outputs raw Lua
    /// (no tool calls, no fenced blocks). Skips tool injection and uses the
    /// training system prompt for ConstructMode.
    pub finetuned_lua: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmBackend {
    OpenAiCompatible,
    Anthropic,
    Mock,
    #[cfg(target_arch = "wasm32")]
    WebLlm,
    Fallback,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: LlmBackend::Mock,
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: "devstral-small-2-2512".into(),
            temperature: 0.2,
            finetuned_lua: false,
        }
    }
}

impl LlmConfig {
    /// Build config from environment variables, falling back to defaults (Mock backend).
    ///
    /// - `CLAWED_LLM_BACKEND`: "openai" | "anthropic" | "mock" | "fallback"
    /// - `CLAWED_LLM_URL`: base URL (default: "http://localhost:11434")
    /// - `CLAWED_API_KEY`: API key (default: empty)
    /// - `CLAWED_LLM_MODEL`: model ID (default: "devstral-small-2-2512")
    /// - `CLAWED_LLM_TEMP`: temperature (default: 0.2)
    /// - `CLAWED_LLM_FINETUNED`: "1" or "true" to enable fine-tuned Lua mode
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(backend) = std::env::var("CLAWED_LLM_BACKEND") {
            config.backend = match backend.to_lowercase().as_str() {
                "openai" => LlmBackend::OpenAiCompatible,
                "anthropic" => LlmBackend::Anthropic,
                "fallback" => LlmBackend::Fallback,
                _ => LlmBackend::Mock,
            };
        }

        if let Ok(url) = std::env::var("CLAWED_LLM_URL") {
            config.base_url = url;
        }

        if let Ok(key) = std::env::var("CLAWED_API_KEY") {
            config.api_key = key;
        }

        if let Ok(model) = std::env::var("CLAWED_LLM_MODEL") {
            config.model = model;
        }

        if let Ok(temp) = std::env::var("CLAWED_LLM_TEMP") {
            if let Ok(t) = temp.parse::<f32>() {
                config.temperature = t;
            }
        }

        if let Ok(ft) = std::env::var("CLAWED_LLM_FINETUNED") {
            config.finetuned_lua = matches!(ft.as_str(), "1" | "true" | "yes");
        }

        config
    }
}

/// Agent readiness status, used by UI to show initialization progress.
#[derive(Debug, Clone, Resource)]
pub enum AgentStatus {
    Unconfigured,
    Initializing(f32),
    Ready,
    Error(String),
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Unconfigured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_defaults_to_mock() {
        // SAFETY: Tests run single-threaded with --test-threads=1 or are
        // isolated by unique env var names that no other test uses.
        unsafe {
            std::env::remove_var("CLAWED_LLM_BACKEND");
            std::env::remove_var("CLAWED_LLM_URL");
            std::env::remove_var("CLAWED_API_KEY");
            std::env::remove_var("CLAWED_LLM_MODEL");
            std::env::remove_var("CLAWED_LLM_TEMP");
            std::env::remove_var("CLAWED_LLM_FINETUNED");
        }

        let config = LlmConfig::from_env();
        assert_eq!(config.backend, LlmBackend::Mock);
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "devstral-small-2-2512");
        assert!((config.temperature - 0.2).abs() < f32::EPSILON);
        assert!(!config.finetuned_lua);
    }

    #[test]
    fn from_env_reads_backend() {
        // SAFETY: Unique env var name, no concurrent mutation.
        unsafe {
            std::env::set_var("CLAWED_LLM_BACKEND", "openai");
        }
        let config = LlmConfig::from_env();
        assert_eq!(config.backend, LlmBackend::OpenAiCompatible);
        unsafe {
            std::env::remove_var("CLAWED_LLM_BACKEND");
        }
    }

    #[test]
    fn from_env_reads_finetuned() {
        unsafe {
            std::env::set_var("CLAWED_LLM_FINETUNED", "1");
        }
        let config = LlmConfig::from_env();
        assert!(config.finetuned_lua);
        unsafe {
            std::env::set_var("CLAWED_LLM_FINETUNED", "true");
        }
        let config2 = LlmConfig::from_env();
        assert!(config2.finetuned_lua);
        unsafe {
            std::env::set_var("CLAWED_LLM_FINETUNED", "0");
        }
        let config3 = LlmConfig::from_env();
        assert!(!config3.finetuned_lua);
        unsafe {
            std::env::remove_var("CLAWED_LLM_FINETUNED");
        }
    }
}
