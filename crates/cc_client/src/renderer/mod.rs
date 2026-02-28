pub mod autotile;
pub mod box_select;
pub mod camera;
pub mod death;
pub mod health_bars;
pub mod minimap;
pub mod props;
pub mod selection;
pub mod terrain_atlas;
pub mod terrain_borders;
pub mod tilemap;
pub mod units;
pub mod water;

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<terrain_atlas::TerrainAtlas>()
            .add_systems(
                Startup,
                tilemap::spawn_tilemap.after(crate::setup::setup_game),
            )
            .add_systems(Startup, props::spawn_props.after(tilemap::spawn_tilemap))
            .add_systems(Startup, minimap::setup_minimap.after(tilemap::spawn_tilemap))
            .add_systems(
                Update,
                (
                    camera::camera_system,
                    units::sync_unit_sprites,
                    selection::render_selection_indicators,
                    health_bars::spawn_health_bars,
                    health_bars::update_health_bars,
                    death::isolate_dead_material,
                    death::death_fade_system.after(death::isolate_dead_material),
                    terrain_borders::draw_terrain_borders,
                    water::animate_water,
                    selection::pulse_selection_rings,
                    minimap::update_minimap,
                    box_select::render_box_select,
                ),
            );
    }
}
