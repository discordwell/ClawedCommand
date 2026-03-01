//! WebLLM client — runs LLM inference in-browser via WebGPU.
//!
//! Bridges to JavaScript's `window.ccWebLLM` namespace via wasm-bindgen.
//! Only compiled on wasm32 targets.

use async_trait::async_trait;
use wasm_bindgen::prelude::*;

use crate::llm_client::{ChatMessage, LlmClient, LlmError, LlmResponse, ToolDef, parse_openai_response};

// JS bindings to window.ccWebLLM.*
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "isAvailable")]
    fn webllm_is_available() -> bool;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "status")]
    fn webllm_status() -> String;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "downloadProgress")]
    fn webllm_download_progress() -> f64;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "init", catch)]
    async fn webllm_init(model_id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "ccWebLLM"], js_name = "complete", catch)]
    async fn webllm_complete(messages_json: &str, tools_json: &str) -> Result<JsValue, JsValue>;
}

/// Check if WebGPU is available in this browser.
pub fn webgpu_available() -> bool {
    webllm_is_available()
}

/// Get the current WebLLM status string.
pub fn status() -> String {
    webllm_status()
}

/// Get model download progress (0.0 to 1.0).
pub fn download_progress() -> f32 {
    webllm_download_progress() as f32
}

/// Initialize WebLLM with the given model.
pub async fn init(model_id: &str) -> Result<(), String> {
    webllm_init(model_id)
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// LLM client backed by WebLLM (in-browser WebGPU inference).
pub struct WebLlmClient {
    model_id: String,
}

impl WebLlmClient {
    pub fn new(model_id: String) -> Self {
        Self { model_id }
    }
}

#[async_trait(?Send)]
impl LlmClient for WebLlmClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let messages_json =
            serde_json::to_string(messages).map_err(|e| LlmError::Parse(e.to_string()))?;

        let tools_json = match tools {
            Some(tools) => {
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
                serde_json::to_string(&tool_defs).map_err(|e| LlmError::Parse(e.to_string()))?
            }
            None => "[]".to_string(),
        };

        let result = webllm_complete(&messages_json, &tools_json)
            .await
            .map_err(|e| LlmError::Api(format!("{:?}", e)))?;

        let json_str = result
            .as_string()
            .ok_or_else(|| LlmError::Parse("WebLLM returned non-string".into()))?;

        let json: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| LlmError::Parse(e.to_string()))?;

        parse_openai_response(&json)
    }

    fn model_name(&self) -> &str {
        &self.model_id
    }
}
