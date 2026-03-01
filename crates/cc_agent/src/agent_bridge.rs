use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};

use cc_core::commands::GameCommand;
use cc_sim::resources::CommandQueue;

use crate::tool_tier::ToolTier;

/// Message from Bevy → background LLM thread.
#[derive(Debug)]
pub struct AgentRequest {
    pub player_id: u8,
    pub prompt: String,
    pub tier: ToolTier,
}

/// Message from background LLM thread → Bevy.
#[derive(Debug)]
pub struct AgentResponse {
    pub content: String,
    pub commands: Vec<GameCommand>,
    pub error: Option<String>,
}

/// Bridge between Bevy (sync) and the background LLM runtime (async).
#[derive(Resource)]
pub struct AgentBridge {
    pub request_tx: Sender<AgentRequest>,
    pub response_rx: Receiver<AgentResponse>,
}

impl Default for AgentBridge {
    fn default() -> Self {
        let (request_tx, _request_rx) = unbounded();
        let (_response_tx, response_rx) = unbounded();
        Self {
            request_tx,
            response_rx,
        }
    }
}

/// Bevy system: poll for LLM responses and push any commands.
pub fn poll_agent_responses(
    bridge: Res<AgentBridge>,
    mut cmd_queue: ResMut<CommandQueue>,
) {
    while let Ok(response) = bridge.response_rx.try_recv() {
        if let Some(err) = &response.error {
            log::warn!("Agent error: {err}");
        }

        for cmd in response.commands {
            cmd_queue.push(cmd);
        }

        if !response.content.is_empty() {
            log::info!("Agent: {}", response.content);
        }
    }
}
