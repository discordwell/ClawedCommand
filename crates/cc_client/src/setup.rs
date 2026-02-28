use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map_format::ResourceKind;
use cc_core::map_gen::{self, MapGenParams};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_core::unit_stats::base_stats;
use cc_sim::resources::{MapResource, PlayerResources};

/// Marker to distinguish unit meshes from tile meshes in queries.
#[derive(Component)]
pub struct UnitMesh;

/// Marker for the dark outline child entity behind a unit.
#[derive(Component)]
pub struct UnitOutline;

/// Shared team color materials for units.
#[derive(Resource)]
pub struct TeamMaterials {
    pub player: Handle<ColorMaterial>,
    pub enemy: Handle<ColorMaterial>,
    pub selected: Handle<ColorMaterial>,
    pub outline: Handle<ColorMaterial>,
}

/// Shared meshes and materials for anime-style cat units.
#[derive(Resource)]
struct AnimeAssets {
    body: Handle<Mesh>,
    outline: Handle<Mesh>,
    ear_left: Handle<Mesh>,
    ear_right: Handle<Mesh>,
    eye_white: Handle<Mesh>,
    pupil: Handle<Mesh>,
    eye_shine: Handle<Mesh>,
    nose: Handle<Mesh>,
    ear_player: Handle<ColorMaterial>,
    ear_enemy: Handle<ColorMaterial>,
    eye_white_mat: Handle<ColorMaterial>,
    pupil_mat: Handle<ColorMaterial>,
    shine_mat: Handle<ColorMaterial>,
    nose_mat: Handle<ColorMaterial>,
}

/// Set up the initial game state: procedurally generated map, camera, starter units.
pub fn setup_game(
    mut commands: Commands,
    mut map_res: ResMut<MapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut player_resources: ResMut<PlayerResources>,
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

    commands.spawn((Camera2d, Transform::from_translation(cam_pos)));

    // Shared team materials
    let team_materials = TeamMaterials {
        player: materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.4, 0.9))),
        enemy: materials.add(ColorMaterial::from_color(Color::srgb(0.9, 0.2, 0.2))),
        selected: materials.add(ColorMaterial::from_color(Color::srgb(0.3, 0.8, 1.0))),
        outline: materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5))),
    };

    // Anime cat shared meshes and feature materials
    let anime = AnimeAssets {
        body: meshes.add(Circle::new(12.0)),
        outline: meshes.add(Circle::new(14.0)),
        ear_left: meshes.add(Triangle2d::new(
            Vec2::new(-9.0, 7.0),
            Vec2::new(-5.0, 16.0),
            Vec2::new(-1.0, 7.0),
        )),
        ear_right: meshes.add(Triangle2d::new(
            Vec2::new(1.0, 7.0),
            Vec2::new(5.0, 16.0),
            Vec2::new(9.0, 7.0),
        )),
        eye_white: meshes.add(Circle::new(3.5)),
        pupil: meshes.add(Circle::new(2.0)),
        eye_shine: meshes.add(Circle::new(0.8)),
        nose: meshes.add(Triangle2d::new(
            Vec2::new(-1.2, 0.0),
            Vec2::new(0.0, -1.5),
            Vec2::new(1.2, 0.0),
        )),
        ear_player: materials.add(ColorMaterial::from_color(Color::srgb(0.15, 0.3, 0.7))),
        ear_enemy: materials.add(ColorMaterial::from_color(Color::srgb(0.7, 0.15, 0.15))),
        eye_white_mat: materials.add(ColorMaterial::from_color(Color::srgb(0.95, 0.95, 0.97))),
        pupil_mat: materials.add(ColorMaterial::from_color(Color::srgb(0.08, 0.08, 0.12))),
        shine_mat: materials.add(ColorMaterial::from_color(Color::srgb(1.0, 1.0, 1.0))),
        nose_mat: materials.add(ColorMaterial::from_color(Color::srgb(0.95, 0.55, 0.6))),
    };

    // --- Spawn resource deposits ---
    let deposit_mesh = meshes.add(Rectangle::new(20.0, 20.0));
    for resource in &map_def.resources {
        let grid = GridPos::new(resource.pos.0, resource.pos.1);
        let world = WorldPos::from_grid(grid);
        let screen = world_to_screen(world);
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;

        let (resource_type, remaining, color) = match resource.kind {
            ResourceKind::FishPond => (ResourceType::Food, 1500, Color::srgb(0.2, 0.6, 0.9)),
            ResourceKind::BerryBush => (ResourceType::Food, 800, Color::srgb(0.8, 0.3, 0.5)),
            ResourceKind::GpuDeposit => (ResourceType::GpuCores, 1000, Color::srgb(0.3, 0.9, 0.3)),
            ResourceKind::MonkeyMine => (ResourceType::Nft, 500, Color::srgb(0.9, 0.7, 0.1)),
        };

        let deposit_mat = materials.add(ColorMaterial::from_color(color));
        commands.spawn((
            Position { world },
            Velocity::zero(),
            GridCell { pos: grid },
            ResourceDeposit { resource_type, remaining },
            Mesh2d(deposit_mesh.clone()),
            MeshMaterial2d(deposit_mat),
            Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world) - 0.1),
        ));
    }

    // --- Spawn anime cat units per player ---
    for sp in &map_def.spawn_points {
        let base_pos = GridPos::new(sp.pos.0, sp.pos.1);

        let unit_configs: [(i32, i32, UnitKind); 6] = [
            (0, 0, UnitKind::Nuisance),
            (1, 0, UnitKind::Nuisance),
            (0, 1, UnitKind::Nuisance),
            (1, 1, UnitKind::Nuisance),
            (-1, 0, UnitKind::Hisser),
            (0, -1, UnitKind::Hisser),
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

            let body_mat = if sp.player == 0 {
                team_materials.player.clone()
            } else {
                team_materials.enemy.clone()
            };
            let ear_mat = if sp.player == 0 {
                anime.ear_player.clone()
            } else {
                anime.ear_enemy.clone()
            };

            let children = spawn_anime_cat_children(&mut commands, &anime, &team_materials, ear_mat);

            commands
                .spawn((
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
                    Mesh2d(anime.body.clone()),
                    MeshMaterial2d(body_mat),
                    Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world))
                        .with_scale(Vec3::splat(scale)),
                ))
                .add_children(&children);
        }
    }

    commands.insert_resource(team_materials);
    commands.insert_resource(anime);
}

/// Spawn child entities for an anime cat: outline, ears, eyes, pupils, shine, nose.
fn spawn_anime_cat_children(
    commands: &mut Commands,
    anime: &AnimeAssets,
    team_mats: &TeamMaterials,
    ear_mat: Handle<ColorMaterial>,
) -> Vec<Entity> {
    let mut c = Vec::with_capacity(10);

    // Outline (behind body)
    c.push(commands.spawn((
        UnitOutline,
        Mesh2d(anime.outline.clone()),
        MeshMaterial2d(team_mats.outline.clone()),
        Transform::from_xyz(0.0, 0.0, -0.01),
    )).id());

    // Ears
    c.push(commands.spawn((
        Mesh2d(anime.ear_left.clone()),
        MeshMaterial2d(ear_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.01),
    )).id());
    c.push(commands.spawn((
        Mesh2d(anime.ear_right.clone()),
        MeshMaterial2d(ear_mat),
        Transform::from_xyz(0.0, 0.0, 0.01),
    )).id());

    // Eyes (white)
    c.push(commands.spawn((
        Mesh2d(anime.eye_white.clone()),
        MeshMaterial2d(anime.eye_white_mat.clone()),
        Transform::from_xyz(-4.0, 2.0, 0.02),
    )).id());
    c.push(commands.spawn((
        Mesh2d(anime.eye_white.clone()),
        MeshMaterial2d(anime.eye_white_mat.clone()),
        Transform::from_xyz(4.0, 2.0, 0.02),
    )).id());

    // Pupils (dark, slightly lower for cute upward gaze)
    c.push(commands.spawn((
        Mesh2d(anime.pupil.clone()),
        MeshMaterial2d(anime.pupil_mat.clone()),
        Transform::from_xyz(-4.0, 1.0, 0.03),
    )).id());
    c.push(commands.spawn((
        Mesh2d(anime.pupil.clone()),
        MeshMaterial2d(anime.pupil_mat.clone()),
        Transform::from_xyz(4.0, 1.0, 0.03),
    )).id());

    // Eye shine (anime sparkle — offset upper-right in each eye)
    c.push(commands.spawn((
        Mesh2d(anime.eye_shine.clone()),
        MeshMaterial2d(anime.shine_mat.clone()),
        Transform::from_xyz(-3.0, 2.5, 0.04),
    )).id());
    c.push(commands.spawn((
        Mesh2d(anime.eye_shine.clone()),
        MeshMaterial2d(anime.shine_mat.clone()),
        Transform::from_xyz(5.0, 2.5, 0.04),
    )).id());

    // Nose (pink triangle)
    c.push(commands.spawn((
        Mesh2d(anime.nose.clone()),
        MeshMaterial2d(anime.nose_mat.clone()),
        Transform::from_xyz(0.0, -2.0, 0.02),
    )).id());

    c
}

/// Scale factor per unit kind.
fn unit_scale(kind: UnitKind) -> f32 {
    match kind {
        UnitKind::Pawdler => 0.7,
        UnitKind::Nuisance => 1.0,
        UnitKind::Mouser => 0.9,
        UnitKind::FerretSapper => 0.9,
        UnitKind::Hisser => 1.0,
        UnitKind::FlyingFox => 0.8,
        UnitKind::Yowler => 1.1,
        UnitKind::Catnapper => 1.3,
        UnitKind::Chonk => 1.4,
        UnitKind::MechCommander => 1.6,
    }
}
