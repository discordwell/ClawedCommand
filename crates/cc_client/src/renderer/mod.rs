pub mod anim_assets;
pub mod animation;
pub mod autotile;
pub mod box_select;
pub mod building_gen;
pub mod buildings;
pub mod camera;
pub mod death;
pub mod fog;
pub mod health_bars;
pub mod hero_sprites;
pub mod minimap;
pub mod projectile_assets;
pub mod projectiles;
pub mod props;
pub mod rally_flag;
pub mod resource_nodes;
pub mod screenshot;
pub mod selection;
pub mod terrain_atlas;
pub mod terrain_borders;
pub mod tile_gen;
pub mod tilemap;
pub mod tweens;
pub mod unit_gen;
pub mod units;
pub mod vfx;
pub mod voice_ping;
pub mod water;
pub mod zoom_lod;

use bevy::prelude::*;

/// Default building sprite/mesh size in pixels (width and height).
pub const BUILDING_SPRITE_SIZE: f32 = 28.0;

/// Check whether an asset file exists on disk relative to the workspace assets/ directory.
/// Returns `false` on WASM (no filesystem access).
#[cfg(not(target_arch = "wasm32"))]
pub fn asset_exists_on_disk(relative_path: &str) -> bool {
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
        .join(relative_path)
        .exists()
}

#[cfg(target_arch = "wasm32")]
pub fn asset_exists_on_disk(_relative_path: &str) -> bool {
    false
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<terrain_atlas::TerrainAtlas>()
            .init_resource::<screenshot::ScreenshotConfig>()
            .init_resource::<fog::FogOfWar>()
            .init_resource::<zoom_lod::ZoomTier>()
            .init_resource::<minimap::MinimapClickConsumed>()
            // Phase 0: Generate sprite assets before anything else (PreStartup)
            .add_systems(
                PreStartup,
                (
                    unit_gen::generate_unit_sprites,
                    resource_nodes::generate_resource_sprites,
                    building_gen::generate_building_sprites,
                    anim_assets::load_anim_assets,
                    projectile_assets::load_projectile_assets,
                    hero_sprites::load_hero_sprites,
                ),
            )
            // Phase 1: Generate terrain tiles at Startup (no map dependency)
            .add_systems(Startup, tile_gen::generate_terrain_tiles)
            // Phase 2: setup_game creates map + spawns units (needs sprite resources)
            // setup_game is registered in main.rs; tilemap runs after both tile_gen and setup_game
            .add_systems(
                Startup,
                tilemap::spawn_tilemap
                    .after(tile_gen::generate_terrain_tiles)
                    .after(crate::setup::setup_game),
            )
            // Phase 3: Systems that depend on the tilemap being spawned
            .add_systems(Startup, props::spawn_props.after(tilemap::spawn_tilemap))
            .add_systems(
                Startup,
                minimap::setup_minimap.after(tilemap::spawn_tilemap),
            )
            .add_systems(
                Startup,
                (
                    fog::init_fog.after(tilemap::spawn_tilemap),
                    fog::spawn_fog_overlays.after(fog::init_fog),
                ),
            )
            .add_systems(
                Update,
                (
                    camera::camera_system,
                    zoom_lod::detect_zoom_tier.after(camera::camera_system),
                    zoom_lod::toggle_lod_visuals
                        .after(zoom_lod::detect_zoom_tier)
                        .run_if(resource_changed::<zoom_lod::ZoomTier>),
                    units::sync_unit_sprites,
                    units::spawn_unit_visuals,
                    buildings::spawn_building_visuals,
                    buildings::sync_building_sprites,
                    buildings::render_placement_preview,
                    selection::render_selection_indicators.after(zoom_lod::detect_zoom_tier),
                    health_bars::spawn_health_bars.run_if(zoom_lod::is_tactical),
                    health_bars::update_health_bars.run_if(zoom_lod::is_tactical),
                    health_bars::hide_dead_health_bars,
                    death::isolate_dead_material,
                    death::death_fade_system
                        .after(death::isolate_dead_material)
                        .run_if(zoom_lod::is_tactical),
                    terrain_borders::draw_terrain_borders.run_if(zoom_lod::is_tactical),
                    water::animate_water.run_if(zoom_lod::is_tactical),
                    selection::pulse_selection_rings.run_if(zoom_lod::is_tactical),
                    minimap::update_minimap,
                    minimap::minimap_click.after(camera::camera_system),
                ),
            )
            // Construction visuals (separate block to avoid tuple size limit)
            .add_systems(
                Update,
                (
                    buildings::spawn_construction_bars.after(buildings::spawn_building_visuals),
                    buildings::update_construction_bars,
                    buildings::remove_construction_bars,
                    buildings::update_construction_alpha_sprite,
                    buildings::update_construction_alpha_mesh,
                    buildings::update_building_damage_tint,
                ),
            )
            .add_systems(
                Update,
                (
                    box_select::render_box_select,
                    rally_flag::rally_flag_system,
                    projectiles::spawn_projectile_sprites,
                    projectiles::sync_projectile_sprites,
                    fog::update_fog_visibility,
                    fog::render_fog_overlays.after(fog::update_fog_visibility),
                    fog::toggle_fog_hotkey,
                    #[cfg(not(target_arch = "wasm32"))]
                    screenshot::screenshot_hotkey,
                    #[cfg(not(target_arch = "wasm32"))]
                    screenshot::screenshot_auto_toggle,
                    #[cfg(not(target_arch = "wasm32"))]
                    screenshot::screenshot_auto_capture,
                ),
            )
            // Animation systems
            .add_systems(
                Update,
                (
                    animation::derive_anim_state,
                    animation::advance_animation.after(animation::derive_anim_state),
                    tweens::apply_unit_tweens
                        .after(units::sync_unit_sprites)
                        .after(selection::render_selection_indicators)
                        .after(animation::advance_animation)
                        .run_if(zoom_lod::is_tactical),
                ),
            )
            // VFX particle systems
            .add_systems(
                Update,
                (
                    vfx::update_particles,
                    vfx::update_emitters,
                    vfx::spawn_trail_particles.run_if(zoom_lod::is_tactical),
                    vfx::spawn_impact_vfx.run_if(zoom_lod::is_tactical),
                ),
            )
            // Voice-command sonar-ping VFX
            .add_systems(
                Update,
                (
                    voice_ping::update_voice_pings,
                    #[cfg(feature = "native")]
                    voice_ping::spawn_voice_pings_from_events,
                ),
            );
    }
}
