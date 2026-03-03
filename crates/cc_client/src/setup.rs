use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map_format::ResourceKind;
use cc_core::map_gen::{self, MapGenParams};
use cc_core::mission::MissionMap;
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_core::unit_stats::base_stats;
use cc_sim::campaign::state::CampaignState;
use cc_sim::resources::{MapResource, PlayerResources, SpawnPositions};

use crate::renderer::animation::{AnimIndices, AnimState, AnimTimer, PrevAnimState};
use crate::renderer::building_gen::{BuildingSprites, building_kind_index, building_scale};
use crate::renderer::tweens::TweenState;
use crate::renderer::buildings::SpriteBuilding;
use crate::renderer::resource_nodes::ResourceNodeSprites;
use crate::renderer::unit_gen::{UnitSprites, kind_index};
use crate::cutscene::CutsceneCamera;
use crate::renderer::zoom_lod::{self, ZoomTier};

/// Marker to distinguish unit entities from tile entities in queries.
#[derive(Component)]
pub struct UnitMesh;

/// Marker for building entities in the renderer.
#[derive(Component)]
pub struct BuildingMesh;

/// Marker for the dark outline child entity behind a unit (kept for compatibility).
#[derive(Component)]
pub struct UnitOutline;

/// Shared team color materials for units (used by selection rings).
#[derive(Resource)]
pub struct TeamMaterials {
    pub player: Handle<ColorMaterial>,
    pub enemy: Handle<ColorMaterial>,
    pub selected: Handle<ColorMaterial>,
    pub outline: Handle<ColorMaterial>,
}

/// Team color tints for sprite-based units.
/// Values are close to white with a color bias so dark sprites stay visible
/// (Bevy's Sprite.color is multiplicative — strong tints crush dark pixels).
pub fn team_color(player_id: u8) -> Color {
    match player_id {
        0 => Color::srgb(0.7, 0.8, 1.0),  // catGPT — soft blue
        1 => Color::srgb(1.0, 0.7, 0.7),  // Murder — soft red
        2 => Color::srgb(1.0, 0.88, 0.6), // Clawed — warm amber
        3 => Color::srgb(0.65, 0.95, 0.7), // Seekers — forest green
        4 => Color::srgb(0.6, 0.95, 0.95), // Croak — teal
        5 => Color::srgb(1.0, 0.78, 0.5), // LLAMA — orange
        _ => Color::srgb(1.0, 0.7, 0.7),  // default — soft red
    }
}

/// Building mesh color by player.
pub fn building_color(player_id: u8) -> Color {
    match player_id {
        0 => Color::srgb(0.3, 0.5, 0.9), // catGPT — blue
        1 => Color::srgb(0.9, 0.3, 0.3), // Murder — red
        2 => Color::srgb(0.8, 0.6, 0.2), // Clawed — amber
        3 => Color::srgb(0.2, 0.7, 0.3), // Seekers — green
        4 => Color::srgb(0.2, 0.7, 0.7), // Croak — teal
        5 => Color::srgb(0.9, 0.5, 0.2), // LLAMA — orange
        _ => Color::srgb(0.9, 0.3, 0.3), // default — red
    }
}

/// Build a GameMap from inline mission tile data.
pub(crate) fn game_map_from_inline(
    width: u32,
    height: u32,
    tiles: &[cc_core::terrain::TerrainType],
    elevation: &[u8],
) -> cc_core::map::GameMap {
    let mut map = cc_core::map::GameMap::new(width, height);
    for (i, (terrain, elev)) in tiles.iter().zip(elevation.iter()).enumerate() {
        let x = (i as u32 % width) as i32;
        let y = (i as u32 / width) as i32;
        if let Some(tile) = map.get_mut(GridPos::new(x, y)) {
            tile.terrain = *terrain;
            tile.elevation = *elev;
        }
    }
    map
}

/// Set a single tile in an inline map tile/elevation array.
pub(crate) fn set_tile(
    tiles: &mut [cc_core::terrain::TerrainType],
    elevation: &mut [u8],
    x: i32,
    y: i32,
    terrain: cc_core::terrain::TerrainType,
    elev: u8,
    width: u32,
    height: u32,
) {
    if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
        let idx = y as usize * width as usize + x as usize;
        tiles[idx] = terrain;
        elevation[idx] = elev;
    }
}

/// Set up the initial game state: procedurally generated map, camera, starter units + base.
pub fn setup_game(
    mut commands: Commands,
    mut map_res: ResMut<MapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut player_resources: ResMut<PlayerResources>,
    mut spawn_positions: ResMut<SpawnPositions>,
    unit_sprites: Option<Res<UnitSprites>>,
    resource_sprites: Option<Res<ResourceNodeSprites>>,
    building_sprites: Option<Res<BuildingSprites>>,
    tier: Res<ZoomTier>,
    campaign: Res<CampaignState>,
    cutscene_cam: Option<Res<CutsceneCamera>>,
) {
    let map_def = if let Some(ref mission) = campaign.current_mission {
        // Build map from mission definition
        match &mission.map {
            MissionMap::Inline { width, height, tiles, elevation } => {
                let map = game_map_from_inline(*width, *height, tiles, elevation);
                map_res.map = map;
                None // No MapDefinition — skip resource deposits
            }
            MissionMap::Generated { seed, .. } => {
                let params = MapGenParams {
                    num_players: 2,
                    seed: *seed,
                    ..Default::default()
                };
                let def = map_gen::generate_map(&params);
                map_res.map = def.to_game_map();
                Some(def)
            }
        }
    } else {
        let params = MapGenParams {
            map_size: cc_core::map_gen::MapSize::Large,
            num_players: 2,
            seed: 42,
            ..Default::default()
        };
        let def = map_gen::generate_map(&params);
        map_res.map = def.to_game_map();
        Some(def)
    };

    // For mission mode, override player resources
    if let Some(ref mission) = campaign.current_mission {
        let setup = &mission.player_setup;
        for pres in player_resources.players.iter_mut() {
            pres.food = setup.starting_food;
            pres.gpu_cores = setup.starting_gpu;
            pres.nfts = setup.starting_nfts;
            pres.supply_cap = 0;
            pres.supply = 0;
        }
    }

    // Camera positioning: cutscene override > showcase heuristic > default center
    let (cam_grid, cam_scale) = if let Some(ref cc) = cutscene_cam {
        (cc.focus, cc.zoom)
    } else {
        let is_showcase = map_res.map.width == 80 && map_res.map.height == 48;
        if is_showcase {
            // Focus on catGPT neighborhood center (13,13) at min zoom for building visibility
            (GridPos::new(13, 13), 0.5)
        } else {
            let cx = map_res.map.width as i32 / 2;
            let cy = map_res.map.height as i32 / 2;
            (GridPos::new(cx, cy), 1.2)
        }
    };
    let center_world = WorldPos::from_grid(cam_grid);
    let center_screen = world_to_screen(center_world);
    let cam_pos = Vec3::new(center_screen.x, -center_screen.y, 0.0);

    commands.spawn((
        Camera2d,
        Transform::from_translation(cam_pos),
        Projection::Orthographic(OrthographicProjection {
            scale: cam_scale,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Shared team materials (still needed for selection rings)
    let team_materials = TeamMaterials {
        player: materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.4, 0.9))),
        enemy: materials.add(ColorMaterial::from_color(Color::srgb(0.9, 0.2, 0.2))),
        selected: materials.add(ColorMaterial::from_color(Color::srgb(0.3, 0.8, 1.0))),
        outline: materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5))),
    };

    // --- Spawn resource deposits (skip in mission mode — no economy) ---
    // --- Record spawn positions and spawn base + units (skip in mission mode — wave_spawner handles it) ---
    if let Some(ref map_def) = map_def {

    for resource in &map_def.resources {
        let grid = GridPos::new(resource.pos.0, resource.pos.1);
        let world = WorldPos::from_grid(grid);
        let screen = world_to_screen(world);
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;

        let (resource_type, remaining) = match resource.kind {
            ResourceKind::FishPond => (ResourceType::Food, 1500),
            ResourceKind::BerryBush => (ResourceType::Food, 800),
            ResourceKind::GpuDeposit => (ResourceType::GpuCores, 1000),
            ResourceKind::MonkeyMine => (ResourceType::Nft, 500),
        };

        if let Some(ref res_sprites) = resource_sprites {
            commands.spawn((
                Position { world },
                Velocity::zero(),
                GridCell { pos: grid },
                ResourceDeposit { resource_type, remaining },
                Sprite {
                    image: res_sprites.get(resource.kind),
                    ..default()
                },
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world) - 0.1),
            ));
        } else {
            // Fallback: colored rectangle
            let color = match resource.kind {
                ResourceKind::FishPond => Color::srgb(0.2, 0.6, 0.9),
                ResourceKind::BerryBush => Color::srgb(0.8, 0.3, 0.5),
                ResourceKind::GpuDeposit => Color::srgb(0.3, 0.9, 0.3),
                ResourceKind::MonkeyMine => Color::srgb(0.9, 0.7, 0.1),
            };
            let deposit_mesh = meshes.add(Rectangle::new(20.0, 20.0));
            let deposit_mat = materials.add(ColorMaterial::from_color(color));
            commands.spawn((
                Position { world },
                Velocity::zero(),
                GridCell { pos: grid },
                ResourceDeposit { resource_type, remaining },
                Mesh2d(deposit_mesh),
                MeshMaterial2d(deposit_mat),
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world) - 0.1),
            ));
        }
    }

    // --- Record spawn positions and spawn base + units per player ---
    let mut total_spawned_per_player = [0u32; 2];

    for sp in &map_def.spawn_points {
        let base_pos = GridPos::new(sp.pos.0, sp.pos.1);

        // Record spawn position for AI
        spawn_positions.positions.push((sp.player, base_pos));

        // --- Spawn TheBox (HQ) at spawn point center ---
        let box_world = WorldPos::from_grid(base_pos);
        let box_screen = world_to_screen(box_world);
        let box_elev = map_res.map.elevation_at(base_pos) as f32 * ELEVATION_PIXEL_OFFSET;
        let bstats = building_stats(BuildingKind::TheBox);

        if let Some(ref bsprites) = building_sprites {
            let idx = building_kind_index(BuildingKind::TheBox);
            let image = bsprites.sprites[idx].clone();
            let has_art = bsprites.has_art.get(idx).copied().unwrap_or(false);
            let scale = building_scale(BuildingKind::TheBox, has_art);
            let tint = team_color(sp.player);
            commands.spawn((
                Position { world: box_world },
                Velocity::zero(),
                GridCell { pos: base_pos },
                Owner { player_id: sp.player },
                Building { kind: BuildingKind::TheBox },
                Health { current: bstats.health, max: bstats.health },
                Producer,
                ProductionQueue::default(),
                BuildingMesh,
                SpriteBuilding,
                Sprite {
                    image,
                    color: tint,
                    ..default()
                },
                Transform::from_xyz(box_screen.x, -box_screen.y + box_elev, depth_z(box_world) - 0.05)
                    .with_scale(Vec3::splat(scale)),
            ));
        } else {
            let box_mesh = meshes.add(Rectangle::new(
                crate::renderer::BUILDING_SPRITE_SIZE,
                crate::renderer::BUILDING_SPRITE_SIZE,
            ));
            let box_mat = materials.add(ColorMaterial::from_color(building_color(sp.player)));
            commands.spawn((
                Position { world: box_world },
                Velocity::zero(),
                GridCell { pos: base_pos },
                Owner { player_id: sp.player },
                Building { kind: BuildingKind::TheBox },
                Health { current: bstats.health, max: bstats.health },
                Producer,
                ProductionQueue::default(),
                BuildingMesh,
                Mesh2d(box_mesh),
                MeshMaterial2d(box_mat),
                Transform::from_xyz(box_screen.x, -box_screen.y + box_elev, depth_z(box_world) - 0.05),
            ));
        }

        // Update supply cap for TheBox
        if let Some(pres) = player_resources.players.get_mut(sp.player as usize) {
            pres.supply_cap += bstats.supply_provided;
        }

        // --- Spawn starter units: 4 Pawdlers + 2 Nuisance ---
        let unit_configs: [(i32, i32, UnitKind); 6] = [
            (1, 0, UnitKind::Pawdler),
            (0, 1, UnitKind::Pawdler),
            (-1, 0, UnitKind::Pawdler),
            (0, -1, UnitKind::Pawdler),
            (1, 1, UnitKind::Nuisance),
            (-1, 1, UnitKind::Nuisance),
        ];

        for &(dx, dy, kind) in &unit_configs {
            let grid = GridPos::new(base_pos.x + dx, base_pos.y + dy);
            if !map_res.map.is_passable(grid) {
                continue;
            }

            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
            let stats = base_stats(kind);
            let scale = unit_scale(kind);
            let tint = team_color(sp.player);

            if let Some(ref sprites) = unit_sprites {
                // Sprite-based unit
                let image = sprites.sprites[kind_index(kind)].clone();
                let unit_entity = commands.spawn((
                    Position { world },
                    Velocity::zero(),
                    GridCell { pos: grid },
                    Owner { player_id: sp.player },
                    UnitType { kind },
                    Health { current: stats.health, max: stats.health },
                    MovementSpeed { speed: stats.speed },
                    AttackStats {
                        damage: stats.damage,
                        range: stats.range,
                        attack_speed: stats.attack_speed,
                        cooldown_remaining: 0,
                    },
                    AttackTypeMarker { attack_type: stats.attack_type },
                    UnitMesh,
                    Sprite {
                        image,
                        color: tint,
                        ..default()
                    },
                    Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world))
                        .with_scale(Vec3::splat(scale)),
                )).id();
                commands.entity(unit_entity).insert((
                    AnimState::default(),
                    PrevAnimState::default(),
                    AnimIndices::default(),
                    AnimTimer::default(),
                    TweenState::new(kind),
                ));

                zoom_lod::spawn_strategic_icon(
                    &mut commands, &mut meshes, &mut materials,
                    unit_entity, scale, tint, &tier,
                );
            } else {
                // Fallback: colored circle mesh
                let body_mesh = meshes.add(Circle::new(12.0));
                let body_mat = if sp.player == 0 {
                    team_materials.player.clone()
                } else {
                    team_materials.enemy.clone()
                };
                let unit_entity = commands.spawn((
                    Position { world },
                    Velocity::zero(),
                    GridCell { pos: grid },
                    Owner { player_id: sp.player },
                    UnitType { kind },
                    Health { current: stats.health, max: stats.health },
                    MovementSpeed { speed: stats.speed },
                    AttackStats {
                        damage: stats.damage,
                        range: stats.range,
                        attack_speed: stats.attack_speed,
                        cooldown_remaining: 0,
                    },
                    AttackTypeMarker { attack_type: stats.attack_type },
                    UnitMesh,
                    Mesh2d(body_mesh),
                    MeshMaterial2d(body_mat),
                    Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world))
                        .with_scale(Vec3::splat(scale)),
                )).id();

                zoom_lod::spawn_strategic_icon(
                    &mut commands, &mut meshes, &mut materials,
                    unit_entity, scale, tint, &tier,
                );
            }

            if (sp.player as usize) < total_spawned_per_player.len() {
                total_spawned_per_player[sp.player as usize] += 1;
            }
        }
    }

    // Set initial supply count to match spawned units
    for (i, &count) in total_spawned_per_player.iter().enumerate() {
        if i < player_resources.players.len() {
            player_resources.players[i].supply = count;
        }
    }

    } // end if let Some(map_def) — skipped in mission mode

    commands.insert_resource(team_materials);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::terrain::TerrainType;

    #[test]
    fn game_map_from_inline_basic() {
        let tiles = vec![
            TerrainType::Grass, TerrainType::Rock, TerrainType::Water,
            TerrainType::Road, TerrainType::Forest, TerrainType::Shallows,
        ];
        let elevation = vec![0, 2, 0, 1, 1, 0];
        let map = game_map_from_inline(3, 2, &tiles, &elevation);
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 2);
        // (0,0) = Grass, elev 0
        assert_eq!(map.terrain_at(GridPos::new(0, 0)), Some(TerrainType::Grass));
        assert_eq!(map.elevation_at(GridPos::new(0, 0)), 0);
        // (1,0) = Rock, elev 2
        assert_eq!(map.terrain_at(GridPos::new(1, 0)), Some(TerrainType::Rock));
        assert_eq!(map.elevation_at(GridPos::new(1, 0)), 2);
        // (2,0) = Water, elev 0
        assert_eq!(map.terrain_at(GridPos::new(2, 0)), Some(TerrainType::Water));
        // (0,1) = Road, elev 1
        assert_eq!(map.terrain_at(GridPos::new(0, 1)), Some(TerrainType::Road));
        assert_eq!(map.elevation_at(GridPos::new(0, 1)), 1);
        // Passability
        assert!(map.is_passable(GridPos::new(0, 0)));  // Grass
        assert!(!map.is_passable(GridPos::new(1, 0))); // Rock
        assert!(!map.is_passable(GridPos::new(2, 0))); // Water (base)
        assert!(map.is_passable(GridPos::new(0, 1)));  // Road
    }

    #[test]
    fn game_map_from_inline_demo_canyon() {
        let ron_str = include_str!("../../../assets/campaign/demo_canyon.ron");
        let mission: cc_core::mission::MissionDefinition = ron::from_str(ron_str).unwrap();
        let cc_core::mission::MissionMap::Inline { width, height, tiles, elevation } = &mission.map else {
            panic!("Expected Inline");
        };
        let map = game_map_from_inline(*width, *height, tiles, elevation);
        assert_eq!(map.width, 80);
        assert_eq!(map.height, 48);
        // P0 HQ position (10,10) should be passable grass
        assert!(map.is_passable(GridPos::new(10, 10)));
        assert_eq!(map.elevation_at(GridPos::new(10, 10)), 1);
        // P1 HQ position (70,38) should be passable grass
        assert!(map.is_passable(GridPos::new(70, 38)));
        assert_eq!(map.elevation_at(GridPos::new(70, 38)), 1);
        // Top wall is rock
        assert_eq!(map.terrain_at(GridPos::new(40, 0)), Some(TerrainType::Rock));
        assert_eq!(map.elevation_at(GridPos::new(40, 0)), 2);
        // River center is water
        assert_eq!(map.terrain_at(GridPos::new(30, 23)), Some(TerrainType::Water));
    }
}

/// Scale factor per unit kind (for 128px art sprites).
/// Sprite images are 128px; tiles are 64px wide.
/// scale = (fraction_of_tile × 64) / 128 = fraction / 2.
/// Target range: 1/3 tile (workers) → full tile (heroes).
pub fn unit_scale(kind: UnitKind) -> f32 {
    match kind {
            // Cat units
            UnitKind::Pawdler => 0.167,         // worker — 1/3 tile
            UnitKind::Nuisance => 0.19,          // harasser — 3/8 tile
            UnitKind::Mouser => 0.19,            // stealth — 3/8 tile
            UnitKind::FerretSapper => 0.25,      // demo — 1/2 tile
            UnitKind::Hisser => 0.25,            // ranged — 1/2 tile
            UnitKind::FlyingFox => 0.25,         // air — 1/2 tile
            UnitKind::Yowler => 0.31,            // support — 5/8 tile
            UnitKind::Catnapper => 0.375,        // siege — 3/4 tile
            UnitKind::Chonk => 0.375,            // tank — 3/4 tile
            UnitKind::MechCommander => 0.50,     // hero — full tile
            // Clawed (mice) — small faction, slightly smaller overall
            UnitKind::Nibblet => 0.167,          // worker
            UnitKind::Swarmer => 0.167,          // swarm — tiny
            UnitKind::Gnawer => 0.19,            // light melee
            UnitKind::Shrieker => 0.25,          // ranged
            UnitKind::Tunneler => 0.25,          // medium
            UnitKind::Sparks => 0.25,            // medium
            UnitKind::Quillback => 0.31,         // heavy
            UnitKind::Whiskerwitch => 0.31,      // support
            UnitKind::Plaguetail => 0.31,        // specialist
            UnitKind::WarrenMarshal => 0.50,     // hero
            // Murder (corvids) — aerial, fragile
            UnitKind::MurderScrounger => 0.167,  // worker
            UnitKind::Sentinel => 0.19,          // scout
            UnitKind::Rookclaw => 0.25,          // melee
            UnitKind::Magpike => 0.25,           // ranged
            UnitKind::Magpyre => 0.25,           // caster
            UnitKind::Jaycaller => 0.25,         // medium
            UnitKind::Jayflicker => 0.25,        // medium
            UnitKind::Dusktalon => 0.31,         // heavy
            UnitKind::Hootseer => 0.375,         // support — large owl
            UnitKind::CorvusRex => 0.50,         // hero
            // Seekers (badgers) — heavy faction, generally larger
            UnitKind::Delver => 0.19,            // worker — badgers are bigger
            UnitKind::Ironhide => 0.31,          // heavy
            UnitKind::Cragback => 0.375,         // tank
            UnitKind::Warden => 0.25,            // medium
            UnitKind::Sapjaw => 0.25,            // medium
            UnitKind::Wardenmother => 0.375,     // heavy support
            UnitKind::SeekerTunneler => 0.25,    // medium
            UnitKind::Embermaw => 0.31,          // heavy ranged
            UnitKind::Dustclaw => 0.25,          // medium
            UnitKind::Gutripper => 0.50,         // hero
            // Croak (axolotls) — medium, regenerating
            UnitKind::Ponderer => 0.167,         // worker
            UnitKind::Regeneron => 0.19,         // light
            UnitKind::Broodmother => 0.31,       // large support
            UnitKind::Gulper => 0.31,            // heavy
            UnitKind::Eftsaber => 0.25,          // medium melee
            UnitKind::Croaker => 0.25,           // medium ranged
            UnitKind::Leapfrog => 0.25,          // medium
            UnitKind::Shellwarden => 0.31,       // heavy defense
            UnitKind::Bogwhisper => 0.31,        // support
            UnitKind::MurkCommander => 0.50,     // hero
            // LLAMA (raccoons) — medium, scrappy
            UnitKind::Scrounger => 0.167,        // worker
            UnitKind::Bandit => 0.19,            // light
            UnitKind::HeapTitan => 0.375,        // tank
            UnitKind::GlitchRat => 0.19,         // light scout
            UnitKind::PatchPossum => 0.25,       // medium
            UnitKind::GreaseMonkey => 0.25,      // medium
            UnitKind::DeadDropUnit => 0.25,      // medium
            UnitKind::Wrecker => 0.31,           // heavy
            UnitKind::DumpsterDiver => 0.31,     // specialist
            UnitKind::JunkyardKing => 0.50,      // hero
    }
}
