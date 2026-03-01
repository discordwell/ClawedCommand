pub mod agent_bridge;
pub mod behaviors;
pub mod construct_mode;
pub mod events;
pub mod llm_client;
pub mod lua_runtime;
pub mod mcp_tools;
pub mod runner;
pub mod script_context;
pub mod snapshot;
pub mod spatial;
pub mod tool_tier;

#[cfg(test)]
pub(crate) mod test_fixtures;

use bevy::prelude::*;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<construct_mode::ConstructModeState>()
            .init_resource::<construct_mode::ScriptLibrary>()
            .init_resource::<agent_bridge::AgentBridge>()
            .init_resource::<tool_tier::ToolRegistry>()
            .init_resource::<tool_tier::FactionToolStates>()
            .add_plugins(runner::ScriptRunnerPlugin)
            .add_systems(Update, agent_bridge::poll_agent_responses)
            .add_systems(FixedUpdate, tool_tier::update_tool_tiers);
    }
}
