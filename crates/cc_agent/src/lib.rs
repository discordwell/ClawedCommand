pub mod agent_bridge;
#[cfg(feature = "harness")]
pub mod arena;
pub mod behaviors;
pub mod construct_mode;
pub mod decision;
pub mod events;
pub mod llm_client;
pub mod mcp_tools;
pub mod snapshot;
pub mod spatial;
pub mod tool_tier;

pub mod script_context;

// Native-only modules (depend on mlua/tokio/crossbeam)
#[cfg(not(target_arch = "wasm32"))]
pub mod llm_runner;
#[cfg(not(target_arch = "wasm32"))]
pub mod lua_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod runner;

// WASM-only modules
#[cfg(target_arch = "wasm32")]
pub mod fallback_client;
#[cfg(target_arch = "wasm32")]
pub mod wasm_runner;
#[cfg(target_arch = "wasm32")]
pub mod webllm_client;

#[cfg(test)]
pub(crate) mod test_fixtures;

use bevy::prelude::*;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<construct_mode::ConstructModeState>()
            .insert_resource(construct_mode::ScriptLibrary::with_starters())
            .init_resource::<agent_bridge::AgentBridge>()
            .init_resource::<agent_bridge::AgentChatLog>()
            .init_resource::<tool_tier::ToolRegistry>()
            .init_resource::<tool_tier::FactionToolStates>()
            .init_resource::<decision::AgentDecisionState>()
            .init_resource::<llm_client::AgentStatus>()
            .add_systems(
                Update,
                (
                    agent_bridge::poll_agent_responses,
                    decision::agent_decision_system,
                ),
            )
            .add_systems(FixedUpdate, tool_tier::update_tool_tiers);

        // Native: add Lua script runner + spawn LLM background thread
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(runner::ScriptRunnerPlugin);

        // WASM: spawn async agent loop on browser event loop
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, wasm_runner::init_wasm_agent);
    }
}
