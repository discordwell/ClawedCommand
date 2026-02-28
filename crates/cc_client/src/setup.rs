use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map_format::ResourceKind;
use cc_core::map_gen::{self, MapGenParams};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_core::unit_stats::base_stats;
use cc_sim::resources::{MapResource, PlayerResources, SpawnPositions};

use crate::renderer::resource_nodes::ResourceNodeSprites;
use crate::renderer::unit_gen::{UnitSprites, kind_index};
use crate::renderer::zoom_lod::StrategicIcon;

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
pub fn team_color(player_id: u8) -> Color {
    if player_id == 0 {
        Color::srgb(0.4, 0.6, 1.0) // Blue tint
    } else {
        Color::srgb(1.0, 0.4, 0.4) // Red tint
    }
}

/// Building mesh color by player.
pub fn building_color(player_id: u8) -> Color {
    if player_id == 0 {
        Color::srgb(0.3, 0.5, 0.9) // Blue
    } else {
        Color::srgb(0.9, 0.3, 0.3) // Red
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
) {
    let params = MapGenParams {
        width: 64,
        height: 64,
        num_players: 2,
        seed: 42,
        ..Default::default()
    };
    let map_def = map_gen::generate_map(&params);
    let map = map_def.to_game_map();
    map_res.map = map;

    // Center camera on the map center
    let cx = map_def.width as i32 / 2;
    let cy = map_def.height as i32 / 2;
    let center_world = WorldPos::from_grid(GridPos::new(cx, cy));
    let center_screen = world_to_screen(center_world);
    let cam_pos = Vec3::new(center_screen.x, -center_screen.y, 0.0);

    commands.spawn((
        Camera2d,
        Transform::from_translation(cam_pos),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.2,
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

    // --- Spawn resource deposits ---
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

        let box_mesh = meshes.add(Rectangle::new(28.0, 28.0));
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

                // Strategic zoom icon: small colored circle, hidden by default
                let icon_mesh = meshes.add(Circle::new(4.0));
                let icon_mat = materials.add(ColorMaterial::from_color(team_color(sp.player)));
                let icon = commands.spawn((
                    StrategicIcon,
                    Mesh2d(icon_mesh),
                    MeshMaterial2d(icon_mat),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    Visibility::Hidden,
                )).id();
                commands.entity(unit_entity).add_children(&[icon]);
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

                // Strategic zoom icon
                let icon_mesh = meshes.add(Circle::new(4.0));
                let icon_mat = materials.add(ColorMaterial::from_color(team_color(sp.player)));
                let icon = commands.spawn((
                    StrategicIcon,
                    Mesh2d(icon_mesh),
                    MeshMaterial2d(icon_mat),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    Visibility::Hidden,
                )).id();
                commands.entity(unit_entity).add_children(&[icon]);
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

    commands.insert_resource(team_materials);
}

/// Scale factor per unit kind. Halved from original values to compensate for
/// 2× sprite resolution (sprites are now double-sized for crisp close-up zoom).
pub fn unit_scale(kind: UnitKind) -> f32 {
    match kind {
        UnitKind::Pawdler => 0.35,
        UnitKind::Nuisance => 0.5,
        UnitKind::Mouser => 0.45,
        UnitKind::FerretSapper => 0.45,
        UnitKind::Hisser => 0.5,
        UnitKind::FlyingFox => 0.4,
        UnitKind::Yowler => 0.55,
        UnitKind::Catnapper => 0.65,
        UnitKind::Chonk => 0.7,
        UnitKind::MechCommander => 0.8,
    }
}
