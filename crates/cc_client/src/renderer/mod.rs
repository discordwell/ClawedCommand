pub mod autotile;
pub mod camera;
pub mod selection;
pub mod terrain_atlas;
pub mod tilemap;
pub mod units;

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<terrain_atlas::TerrainAtlas>()
            .add_systems(Startup, tilemap::spawn_tilemap)
            .add_systems(
                Update,
                (
                    camera::camera_system,
                    units::sync_unit_sprites,
                    selection::render_selection_indicators,
                ),
            );
    }
}
