use bevy::prelude::*;

use cc_core::components::{GridCell, Owner, UnitType};
use cc_core::coords::{GridPos, WorldPos, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Local player ID for fog-of-war calculations.
const LOCAL_PLAYER: u8 = 0;

/// Per-tile fog of war state.
#[derive(Resource)]
pub struct FogOfWar {
    /// Whether fog rendering is enabled.
    pub enabled: bool,
    /// Which tiles have been explored (seen at least once).
    pub explored: Vec<bool>,
    /// Which tiles are currently visible (in unit vision range).
    pub visible: Vec<bool>,
    /// Default vision range in tiles.
    pub vision_range: u32,
    /// Map dimensions for indexing.
    pub width: u32,
    pub height: u32,
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self {
            enabled: true,
            explored: Vec::new(),
            visible: Vec::new(),
            vision_range: 5,
            width: 0,
            height: 0,
        }
    }
}

impl FogOfWar {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }
}

/// Marker component for fog overlay entities.
#[derive(Component)]
pub struct FogOverlay {
    pub grid_x: i32,
    pub grid_y: i32,
}

/// Initialize the FogOfWar resource based on map dimensions.
pub fn init_fog(mut fog: ResMut<FogOfWar>, map_res: Res<MapResource>) {
    let w = map_res.map.width as u32;
    let h = map_res.map.height as u32;
    let size = (w * h) as usize;
    fog.width = w;
    fog.height = h;
    fog.explored = vec![false; size];
    fog.visible = vec![false; size];
}

/// Spawn fog overlay entities for every tile (runs once after tilemap).
/// Each tile gets its own material so alpha can be set independently.
pub fn spawn_fog_overlays(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;

    let fog_mesh = meshes.add(Rhombus::new(
        cc_core::coords::TILE_HALF_WIDTH * 2.0,
        cc_core::coords::TILE_HALF_HEIGHT * 2.0,
    ));

    // Z=100.0 puts fog above all game entities (units ~0 to -1.3, props ~-5, tiles ~-10)
    // but below UI elements (box select at 999.0).
    const FOG_Z: f32 = 100.0;

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

            // Each tile gets its own material so we can set alpha independently
            let fog_material = materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.85)));

            commands.spawn((
                FogOverlay { grid_x: x, grid_y: y },
                Mesh2d(fog_mesh.clone()),
                MeshMaterial2d(fog_material),
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, FOG_Z),
            ));
        }
    }
}

/// Recompute fog visibility based on player unit positions.
pub fn update_fog_visibility(
    mut fog: ResMut<FogOfWar>,
    units: Query<(&GridCell, &Owner), With<UnitType>>,
) {
    if !fog.enabled {
        return;
    }

    let w = fog.width;
    let h = fog.height;
    if w == 0 || h == 0 {
        return;
    }

    // Clear visible state
    fog.visible.fill(false);

    let range = fog.vision_range as i32;

    // Mark tiles visible around each player unit
    for (grid_cell, owner) in units.iter() {
        if owner.player_id != LOCAL_PLAYER {
            continue;
        }
        let ux = grid_cell.pos.x;
        let uy = grid_cell.pos.y;

        for dy in -range..=range {
            for dx in -range..=range {
                // Circular vision: check distance
                if dx * dx + dy * dy > range * range {
                    continue;
                }
                let tx = ux + dx;
                let ty = uy + dy;
                if let Some(idx) = fog.index(tx, ty) {
                    fog.visible[idx] = true;
                    fog.explored[idx] = true;
                }
            }
        }
    }
}

/// Update fog overlay material alpha based on visibility state.
pub fn render_fog_overlays(
    fog: Res<FogOfWar>,
    query: Query<(&FogOverlay, &MeshMaterial2d<ColorMaterial>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if fog.width == 0 {
        return;
    }

    for (overlay, mat_handle) in query.iter() {
        let Some(idx) = fog.index(overlay.grid_x, overlay.grid_y) else {
            continue;
        };

        let target_alpha = if !fog.enabled {
            0.0
        } else if fog.visible[idx] {
            0.0
        } else if fog.explored[idx] {
            0.45
        } else {
            0.85
        };

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.color.set_alpha(target_alpha);
        }
    }
}

/// F10: Toggle fog of war visibility.
pub fn toggle_fog_hotkey(keyboard: Res<ButtonInput<KeyCode>>, mut fog: ResMut<FogOfWar>) {
    if keyboard.just_pressed(KeyCode::F10) {
        fog.enabled = !fog.enabled;
        info!("Fog of war: {}", if fog.enabled { "on" } else { "off" });
    }
}
