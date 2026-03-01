use async_trait::async_trait;
use wasm_bindgen::prelude::*;

use crate::llm_client::{LlmClient, LlmError, LlmResponse, ChatMessage, ToolDef, parse_openai_response};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], catch)]
    async fn init(model_id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], catch)]
    async fn complete(messages_json: &str, tools_json: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "isAvailable")]
    fn is_available() -> bool;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"])]
    fn status() -> String;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "downloadProgress")]
    fn download_progress() -> f64;
}

/// Check if WebGPU is available in the browser.
pub fn webgpu_available() -> bool {
    is_available()
}

/// Get the current WebLLM status string.
pub fn webllm_status() -> String {
    status()
}

/// Get model download progress (0.0 to 1.0).
pub fn webllm_download_progress() -> f64 {
    download_progress()
}

/// Initialize WebLLM with a model.
pub async fn webllm_init(model_id: &str) -> Result<(), String> {
    init(model_id)
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// LLM client backed by WebLLM (in-browser WebGPU inference).
pub struct WebLlmClient {
    model: String,
}

impl WebLlmClient {
    pub fn new(model: String) -> Self {
        Self { model }
    }
}

#[async_trait(?Send)]
impl LlmClient for WebLlmClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let messages_json = serde_json::to_string(messages)
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        let tools_json = match tools {
            Some(t) => {
                let tool_defs: Vec<serde_json::Value> = t
                    .iter()
                    .map(|td| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": td.name,
                                "description": td.description,
                                "parameters": td.parameters,
                            }
                        })
                    })
                    .collect();
                serde_json::to_string(&tool_defs)
                    .map_err(|e| LlmError::Parse(e.to_string()))?
            }
            None => "[]".to_string(),
        };

        let result = complete(&messages_json, &tools_json)
            .await
            .map_err(|e| LlmError::Api(format!("{:?}", e)))?;

        let response_str = result
            .as_string()
            .ok_or_else(|| LlmError::Parse("WebLLM returned non-string".into()))?;

        let json: serde_json::Value = serde_json::from_str(&response_str)
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        parse_openai_response(&json)
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
