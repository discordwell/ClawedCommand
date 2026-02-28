pub mod ai;
#[cfg(feature = "harness")]
pub mod harness;
pub mod pathfinding;
pub mod resources;
pub mod systems;

use bevy::prelude::*;
use systems::SimSystemsPlugin;

/// Main simulation plugin. Registers all sim resources and systems.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(10.0))
            .add_plugins(SimSystemsPlugin)
            .add_plugins(ai::AiPlugin);
    }
}
