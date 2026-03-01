//! Fallback LLM client — tries multiple providers in order.
//!
//! Used on WASM to chain: WebLLM -> Local (Ollama) -> Remote API.
//! Only compiled on wasm32 targets.

use async_trait::async_trait;

use crate::llm_client::{ChatMessage, LlmClient, LlmError, LlmResponse, ToolDef};

/// Tries providers in order, returning the first successful response.
pub struct FallbackClient {
    providers: Vec<(String, Box<dyn LlmClient>)>,
}

impl FallbackClient {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(mut self, name: String, client: Box<dyn LlmClient>) -> Self {
        self.providers.push((name, client));
        self
    }
}

#[async_trait(?Send)]
impl LlmClient for FallbackClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let mut last_err = LlmError::Api("No providers configured".into());

        for (name, client) in &self.providers {
            match client.complete(messages, tools).await {
                Ok(response) => {
                    log::info!("FallbackClient: succeeded with provider '{}'", name);
                    return Ok(response);
                }
                Err(e) => {
                    log::warn!("FallbackClient: provider '{}' failed: {}", name, e);
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    fn model_name(&self) -> &str {
        self.providers
            .first()
            .map(|(name, _)| name.as_str())
            .unwrap_or("fallback")
    }
}
