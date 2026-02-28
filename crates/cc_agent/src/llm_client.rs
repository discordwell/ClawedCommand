use async_trait::async_trait;
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
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError>;

    fn model_name(&self) -> &str;
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
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(
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

/// LLM configuration resource.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub backend: LlmBackend,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmBackend {
    OpenAiCompatible,
    Anthropic,
    Mock,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: LlmBackend::Mock,
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: "devstral-small-2-2512".into(),
            temperature: 0.2,
        }
    }
}
