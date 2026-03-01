use async_trait::async_trait;

use crate::llm_client::{ChatMessage, LlmClient, LlmError, LlmResponse, ToolDef};

/// An LLM client that tries multiple providers in order.
/// Falls back to the next provider if the current one fails.
pub struct FallbackClient {
    providers: Vec<Box<dyn LlmClient>>,
    label: String,
}

impl FallbackClient {
    pub fn new(providers: Vec<Box<dyn LlmClient>>) -> Self {
        let label = providers
            .iter()
            .map(|p| p.model_name().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        Self { providers, label }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl LlmClient for FallbackClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<LlmResponse, LlmError> {
        let mut last_error = LlmError::Api("No providers configured".into());

        for provider in &self.providers {
            match provider.complete(messages, tools).await {
                Ok(resp) => {
                    log::info!("Fallback: succeeded with {}", provider.model_name());
                    return Ok(resp);
                }
                Err(e) => {
                    log::warn!(
                        "Fallback: provider {} failed: {}, trying next",
                        provider.model_name(),
                        e
                    );
                    last_error = e;
                }
            }
        }

        Err(last_error)
    }

    fn model_name(&self) -> &str {
        &self.label
    }
}
