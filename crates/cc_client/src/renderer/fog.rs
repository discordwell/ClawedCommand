use bevy::prelude::*;

use cc_core::components::{GridCell, Owner, UnitType};
use cc_core::coords::{GridPos, TILE_HALF_HEIGHT, TILE_HALF_WIDTH, WorldPos, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Local player ID for fog-of-war calculations.
const LOCAL_PLAYER: u8 = 0;

/// Per-tile rendering state for dirty tracking.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TileFogState {
    Unexplored,
    Explored,
    Visible,
    Disabled,
}

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
    /// Shared material for unexplored tiles (alpha 0.85).
    mat_unexplored: Handle<ColorMaterial>,
    /// Shared material for explored-but-not-visible tiles (alpha 0.45).
    mat_explored: Handle<ColorMaterial>,
    /// Fog overlay entity per tile for O(1) lookup.
    entities: Vec<Entity>,
    /// Previous frame's computed state per tile (dirty tracking).
    prev_state: Vec<TileFogState>,
    /// Whether fog was enabled last frame.
    prev_enabled: bool,
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
            mat_unexplored: Handle::default(),
            mat_explored: Handle::default(),
            entities: Vec::new(),
            prev_state: Vec::new(),
            prev_enabled: true,
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
pub struct FogOverlay;

/// Initialize the FogOfWar resource and create shared materials.
pub fn init_fog(
    mut fog: ResMut<FogOfWar>,
    map_res: Res<MapResource>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let w = map_res.map.width;
    let h = map_res.map.height;
    let size = (w * h) as usize;
    fog.width = w;
    fog.height = h;
    fog.explored = vec![false; size];
    fog.visible = vec![false; size];
    fog.prev_state = vec![TileFogState::Unexplored; size];
    fog.prev_enabled = true;
    // Only 2 shared materials instead of 4,096 unique ones
    fog.mat_unexplored =
        materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.85)));
    fog.mat_explored = materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.45)));
}

/// Spawn fog overlay entities using shared materials.
pub fn spawn_fog_overlays(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut fog: ResMut<FogOfWar>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let map = &map_res.map;

    let fog_mesh = meshes.add(Rhombus::new(TILE_HALF_WIDTH * 2.0, TILE_HALF_HEIGHT * 2.0));

    // Z=100.0 puts fog above all game entities but below UI elements.
    const FOG_Z: f32 = 100.0;

    let size = (map.width * map.height) as usize;
    fog.entities = Vec::with_capacity(size);

    // All tiles start as unexplored — use shared unexplored material
    let mat = fog.mat_unexplored.clone();

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

            let entity = commands
                .spawn((
                    FogOverlay,
                    Mesh2d(fog_mesh.clone()),
                    MeshMaterial2d(mat.clone()),
                    Transform::from_xyz(screen.x, -screen.y + elevation_offset, FOG_Z),
                ))
                .id();

            fog.entities.push(entity);
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

/// Update fog overlay visibility — only touches tiles whose state changed.
pub fn render_fog_overlays(
    mut fog: ResMut<FogOfWar>,
    mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &mut Visibility), With<FogOverlay>>,
) {
    if fog.width == 0 || fog.entities.is_empty() {
        return;
    }

    let size = (fog.width * fog.height) as usize;
    let enabled_changed = fog.enabled != fog.prev_enabled;
    fog.prev_enabled = fog.enabled;

    // Clone handles outside loop to avoid borrow issues
    let mat_unexplored = fog.mat_unexplored.clone();
    let mat_explored = fog.mat_explored.clone();

    for idx in 0..size {
        let new_state = if !fog.enabled {
            TileFogState::Disabled
        } else if fog.visible[idx] {
            TileFogState::Visible
        } else if fog.explored[idx] {
            TileFogState::Explored
        } else {
            TileFogState::Unexplored
        };

        // Skip unchanged tiles — the key optimization
        if new_state == fog.prev_state[idx] && !enabled_changed {
            continue;
        }
        fog.prev_state[idx] = new_state;

        let entity = fog.entities[idx];
        if let Ok((mut mat_handle, mut visibility)) = query.get_mut(entity) {
            match new_state {
                TileFogState::Disabled | TileFogState::Visible => {
                    *visibility = Visibility::Hidden;
                }
                TileFogState::Explored => {
                    *visibility = Visibility::Inherited;
                    mat_handle.0 = mat_explored.clone();
                }
                TileFogState::Unexplored => {
                    *visibility = Visibility::Inherited;
                    mat_handle.0 = mat_unexplored.clone();
                }
            }
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
