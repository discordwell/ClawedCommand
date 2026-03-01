//! 3D Rendering PoC — Tripo GLB → Bevy 3D Isometric Camera
//!
//! Validates the pipeline: AI-generated GLB model loaded in Bevy with a 3D
//! orthographic isometric camera on a small terrain grid.
//!
//! Controls:
//!   A/D — orbit camera around scene center
//!   Q/E — zoom in/out (orthographic scale)
//!   1-4 — move unit to preset positions
//!   Esc — exit

use bevy::camera::ScalingMode;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PocUnit {
    team_color: Color,
    target: Vec3,
    /// Whether team color has been applied to descendant materials.
    colored: bool,
}

#[derive(Component)]
struct PocCamera {
    /// Current orbit angle in radians around the Y axis.
    orbit_angle: f32,
    /// Distance from the look-at center (controls orbit radius).
    radius: f32,
    /// Orthographic zoom scale.
    zoom: f32,
}

#[derive(Component)]
struct GroundTile;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SCENE_CENTER: Vec3 = Vec3::new(2.0, 0.0, 2.0);
const ORBIT_SPEED: f32 = 1.5; // rad/s
const ZOOM_SPEED: f32 = 2.0;
const MIN_ZOOM: f32 = 4.0;
const MAX_ZOOM: f32 = 20.0;
const MOVE_SPEED: f32 = 3.0;
const GRID_SIZE: i32 = 5;
const TILE_SIZE: f32 = 1.0;

/// Preset positions for 1-4 keys.
const PRESET_POSITIONS: [Vec3; 4] = [
    Vec3::new(0.5, 0.0, 0.5),
    Vec3::new(3.5, 0.0, 0.5),
    Vec3::new(3.5, 0.0, 3.5),
    Vec3::new(0.5, 0.0, 3.5),
];

// ---------------------------------------------------------------------------
// Terrain definition
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum TileKind {
    Grass,
    Dirt,
    Forest,
}

impl TileKind {
    fn color(self) -> Color {
        match self {
            TileKind::Grass => Color::srgb(0.3, 0.65, 0.2),
            TileKind::Dirt => Color::srgb(0.55, 0.38, 0.22),
            TileKind::Forest => Color::srgb(0.15, 0.45, 0.12),
        }
    }
}

/// Returns (tile kind, elevation) for each grid position.
fn terrain_at(x: i32, z: i32) -> (TileKind, f32) {
    // Center tiles are elevated, edges are grass/dirt
    let dx = (x as f32 - 2.0).abs();
    let dz = (z as f32 - 2.0).abs();
    let dist = dx.max(dz);

    if dist < 0.5 {
        (TileKind::Forest, 0.6) // center peak
    } else if dist < 1.5 {
        (TileKind::Grass, 0.3) // inner ring, slightly elevated
    } else {
        if (x + z) % 3 == 0 {
            (TileKind::Dirt, 0.0)
        } else {
            (TileKind::Grass, 0.0)
        }
    }
}

/// Get Y elevation for a world-space XZ position (bilinear-ish snap to nearest tile).
fn elevation_at(x: f32, z: f32) -> f32 {
    let gx = x.round() as i32;
    let gz = z.round() as i32;
    let gx = gx.clamp(0, GRID_SIZE - 1);
    let gz = gz.clamp(0, GRID_SIZE - 1);
    terrain_at(gx, gz).1
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // -- Camera --
    let cam = PocCamera {
        orbit_angle: std::f32::consts::FRAC_PI_4, // 45°
        radius: 14.0,
        zoom: 12.0,
    };
    let cam_pos = camera_position(cam.orbit_angle, cam.radius);

    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: cam.zoom,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(cam_pos).looking_at(SCENE_CENTER, Vec3::Y),
        cam,
    ));

    // -- Directional light (sun) --
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    // -- Ambient light --
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
        affects_lightmapped_meshes: true,
    });

    // -- Ground tiles --
    let tile_mesh = meshes.add(Plane3d::default().mesh().size(TILE_SIZE, TILE_SIZE));
    for x in 0..GRID_SIZE {
        for z in 0..GRID_SIZE {
            let (kind, elevation) = terrain_at(x, z);
            let mat = materials.add(StandardMaterial {
                base_color: kind.color(),
                perceptual_roughness: 0.9,
                ..default()
            });
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(x as f32, elevation, z as f32),
                GroundTile,
            ));

            // Side walls for elevated tiles to make elevation visible
            if elevation > 0.05 {
                let wall_color = Color::srgb(0.35, 0.25, 0.15);
                let wall_mat = materials.add(StandardMaterial {
                    base_color: wall_color,
                    perceptual_roughness: 0.95,
                    ..default()
                });
                let wall_mesh = meshes.add(Cuboid::new(TILE_SIZE, elevation, TILE_SIZE));
                commands.spawn((
                    Mesh3d(wall_mesh),
                    MeshMaterial3d(wall_mat),
                    Transform::from_xyz(x as f32, elevation / 2.0, z as f32),
                ));
            }
        }
    }

    // -- Unit: try to load GLB, fall back to colored cube placeholder --
    let start_pos = Vec3::new(2.0, elevation_at(2.0, 2.0), 2.0);
    let team_blue = Color::srgb(0.2, 0.4, 0.9);

    let glb_path = "models/units/pawdler.glb";
    let glb_exists = std::path::Path::new("assets/models/units/pawdler.glb").exists();

    if glb_exists {
        // Load GLB scene
        commands.spawn((
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(glb_path))),
            Transform::from_translation(start_pos).with_scale(Vec3::splat(0.5)),
            PocUnit {
                team_color: team_blue,
                target: start_pos,
                colored: false,
            },
        ));
    } else {
        // Placeholder cube
        info!("GLB not found at assets/{glb_path}, using placeholder cube");
        let cube_mesh = meshes.add(Cuboid::new(0.4, 0.6, 0.4));
        let cube_mat = materials.add(StandardMaterial {
            base_color: team_blue,
            ..default()
        });
        commands.spawn((
            Mesh3d(cube_mesh),
            MeshMaterial3d(cube_mat),
            Transform::from_translation(start_pos + Vec3::Y * 0.3),
            PocUnit {
                team_color: team_blue,
                target: start_pos,
                colored: true, // already colored
            },
        ));
    }

    info!("PoC 3D scene ready — A/D orbit, Q/E zoom, 1-4 move unit");
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Apply team color tint to descendant meshes of the unit (for loaded GLB).
fn apply_team_color(
    mut unit_q: Query<(Entity, &mut PocUnit)>,
    children_q: Query<&Children>,
    mesh_q: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut unit) in &mut unit_q {
        if unit.colored {
            continue;
        }

        let mut found_any = false;
        // Walk all descendants
        for descendant in children_q.iter_descendants(entity) {
            if let Ok(mat_handle) = mesh_q.get(descendant) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    let tint = unit.team_color.to_linear();
                    let base = mat.base_color.to_linear();
                    // Multiply blend: preserves texture detail while tinting
                    mat.base_color = Color::LinearRgba(LinearRgba::new(
                        base.red * 0.5 + tint.red * 0.5,
                        base.green * 0.5 + tint.green * 0.5,
                        base.blue * 0.5 + tint.blue * 0.5,
                        base.alpha,
                    ));
                    found_any = true;
                }
            }
        }

        if found_any {
            unit.colored = true;
            info!("Team color applied to unit");
        }
    }
}

/// Move unit toward its target position, rotating to face movement direction.
fn unit_movement(time: Res<Time>, mut query: Query<(&mut Transform, &PocUnit)>) {
    let dt = time.delta_secs();

    for (mut transform, unit) in &mut query {
        let diff = unit.target - transform.translation;
        let horizontal = Vec3::new(diff.x, 0.0, diff.z);

        if horizontal.length() < 0.05 {
            // Snap to target, fix Y to elevation
            transform.translation.x = unit.target.x;
            transform.translation.z = unit.target.z;
            transform.translation.y = elevation_at(unit.target.x, unit.target.z);
            continue;
        }

        // Rotate to face movement direction
        let forward = horizontal.normalize();
        let target_rotation = Quat::from_rotation_y((-forward.x).atan2(-forward.z));
        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);

        // Move toward target
        let step = forward * MOVE_SPEED * dt;
        if step.length() > horizontal.length() {
            transform.translation.x = unit.target.x;
            transform.translation.z = unit.target.z;
        } else {
            transform.translation.x += step.x;
            transform.translation.z += step.z;
        }

        // Snap Y to terrain elevation
        transform.translation.y = elevation_at(transform.translation.x, transform.translation.z);
    }
}

/// Handle 1-4 keys to set unit target.
fn unit_input(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut PocUnit>) {
    let key_map = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
    ];

    for (key, idx) in key_map {
        if keyboard.just_pressed(key) {
            for mut unit in &mut query {
                let mut pos = PRESET_POSITIONS[idx];
                pos.y = elevation_at(pos.x, pos.z);
                unit.target = pos;
                info!("Unit moving to preset {}", idx + 1);
            }
        }
    }
}

fn camera_position(orbit_angle: f32, radius: f32) -> Vec3 {
    // Isometric-style: orbit around Y, elevated at ~35° from horizontal
    let elevation_angle: f32 = 0.6154; // ~35.26° in radians (atan(1/√2) — true isometric)
    let y = radius * elevation_angle.sin();
    let horizontal = radius * elevation_angle.cos();
    Vec3::new(
        horizontal * orbit_angle.cos() + SCENE_CENTER.x,
        y + SCENE_CENTER.y,
        horizontal * orbit_angle.sin() + SCENE_CENTER.z,
    )
}

/// Orbit camera with A/D, zoom with Q/E.
fn camera_orbit(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Projection, &mut PocCamera)>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut projection, mut cam) in &mut query {
        // Orbit
        if keyboard.pressed(KeyCode::KeyA) {
            cam.orbit_angle -= ORBIT_SPEED * dt;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            cam.orbit_angle += ORBIT_SPEED * dt;
        }

        // Zoom
        if keyboard.pressed(KeyCode::KeyQ) {
            cam.zoom = (cam.zoom - ZOOM_SPEED * dt).max(MIN_ZOOM);
        }
        if keyboard.pressed(KeyCode::KeyE) {
            cam.zoom = (cam.zoom + ZOOM_SPEED * dt).min(MAX_ZOOM);
        }

        // Apply
        let pos = camera_position(cam.orbit_angle, cam.radius);
        *transform = Transform::from_translation(pos).looking_at(SCENE_CENTER, Vec3::Y);

        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: cam.zoom,
            };
        }
    }
}

/// Exit on Escape.
fn exit_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ClawedCommand — 3D PoC".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                apply_team_color,
                unit_input,
                unit_movement,
                camera_orbit,
                exit_on_esc,
            ),
        )
        .run();
}
