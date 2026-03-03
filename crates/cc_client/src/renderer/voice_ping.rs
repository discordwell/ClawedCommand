use bevy::prelude::*;

use cc_core::coords::{world_to_screen, GridPos, WorldPos};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;

/// Marker + state for an expanding, fading sonar-ping ring.
#[derive(Component)]
pub struct VoicePing {
    pub elapsed: f32,
    pub lifetime: f32,
}

/// Golden ping colour matching the voice-buff tint.
const PING_COLOR: Color = Color::srgba(1.0, 0.85, 0.3, 0.6);
/// Starting scale of the ring.
const START_SCALE: f32 = 0.5;
/// Scale at the end of the lifetime.
const END_SCALE: f32 = 3.5;
/// Default lifetime in seconds.
const PING_LIFETIME: f32 = 1.0;

/// Spawn a sonar-ping ring at the given grid position.
/// `elevation` is the terrain elevation at the target tile (0 if unknown).
pub fn spawn_voice_ping(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    target: GridPos,
    elevation: u8,
) {
    let world = WorldPos::from_grid(target);
    let screen = world_to_screen(world);
    let elev_offset = elevation as f32 * ELEVATION_PIXEL_OFFSET;
    let z = 0.02; // just above ground, below units

    let mesh = meshes.add(Annulus::new(8.0, 11.0));
    let mat = materials.add(ColorMaterial::from_color(PING_COLOR));

    commands.spawn((
        VoicePing {
            elapsed: 0.0,
            lifetime: PING_LIFETIME,
        },
        Mesh2d(mesh),
        MeshMaterial2d::<ColorMaterial>(mat),
        Transform::from_xyz(screen.x, -screen.y + elev_offset, z)
            .with_scale(Vec3::splat(START_SCALE)),
    ));
}

/// Expand and fade voice-ping rings, despawn when expired.
pub fn update_voice_pings(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut pings: Query<(
        Entity,
        &mut VoicePing,
        &mut Transform,
        &MeshMaterial2d<ColorMaterial>,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut ping, mut transform, mat_handle) in pings.iter_mut() {
        ping.elapsed += dt;
        let t = (ping.elapsed / ping.lifetime).min(1.0);

        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Expand outward
        let scale = START_SCALE + t * (END_SCALE - START_SCALE);
        transform.scale = Vec3::splat(scale);

        // Fade out
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let alpha = 0.6 * (1.0 - t);
            mat.color = Color::srgba(1.0, 0.85, 0.3, alpha);
        }
    }
}

/// Consume [`cc_voice::events::VoicePingRequest`] messages and spawn pings.
#[cfg(feature = "native")]
pub fn spawn_voice_pings_from_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut ping_events: MessageReader<cc_voice::events::VoicePingRequest>,
) {
    for event in ping_events.read() {
        spawn_voice_ping(&mut commands, &mut meshes, &mut materials, event.target, event.elevation);
    }
}
