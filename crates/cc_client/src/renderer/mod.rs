pub mod autotile;
pub mod box_select;
pub mod camera;
pub mod death;
pub mod fog;
pub mod health_bars;
pub mod minimap;
pub mod projectiles;
pub mod props;
pub mod resource_nodes;
pub mod screenshot;
pub mod selection;
pub mod terrain_atlas;
pub mod terrain_borders;
pub mod tile_gen;
pub mod tilemap;
pub mod unit_gen;
pub mod units;
pub mod water;

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<terrain_atlas::TerrainAtlas>()
            .init_resource::<screenshot::ScreenshotConfig>()
            .init_resource::<fog::FogOfWar>()
            .add_systems(
                Startup,
                (
                    tile_gen::generate_terrain_tiles,
                    unit_gen::generate_unit_sprites,
                    resource_nodes::generate_resource_sprites,
                ),
            )
            .add_systems(
                Startup,
                // setup_game runs in PreStartup, so map is ready by Startup
                tilemap::spawn_tilemap
                    .after(tile_gen::generate_terrain_tiles),
            )
            .add_systems(Startup, props::spawn_props.after(tilemap::spawn_tilemap))
            .add_systems(Startup, minimap::setup_minimap.after(tilemap::spawn_tilemap))
            .add_systems(
                Startup,
                (
                    fog::init_fog.after(tilemap::spawn_tilemap),
                    fog::spawn_fog_overlays.after(tilemap::spawn_tilemap),
                ),
            )
            .add_systems(
                Update,
                (
                    camera::camera_system,
                    units::sync_unit_sprites,
                    selection::render_selection_indicators,
                    health_bars::spawn_health_bars,
                    health_bars::update_health_bars,
                    health_bars::hide_dead_health_bars,
                    death::isolate_dead_material,
                    death::death_fade_system.after(death::isolate_dead_material),
                    terrain_borders::draw_terrain_borders,
                    water::animate_water,
                    selection::pulse_selection_rings,
                    minimap::update_minimap,
                ),
            )
            .add_systems(
                Update,
                (
                    box_select::render_box_select,
                    projectiles::spawn_projectile_sprites,
                    projectiles::sync_projectile_sprites,
                    fog::update_fog_visibility,
                    fog::render_fog_overlays.after(fog::update_fog_visibility),
                    fog::toggle_fog_hotkey,
                    screenshot::screenshot_hotkey,
                    screenshot::screenshot_auto_toggle,
                    screenshot::screenshot_auto_capture,
                ),
            );
    }
}
