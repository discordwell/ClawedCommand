use bevy::prelude::*;

use cc_core::components::{Building, Owner, Producer, RallyPoint, Selected};
use cc_core::coords::{TILE_HALF_HEIGHT, WorldPos, depth_z, world_to_screen};

/// Local player ID for rally flag rendering.
const LOCAL_PLAYER: u8 = 0;

/// Marker for the rally flag visual entity.
#[derive(Component)]
pub struct RallyFlag;

/// Show/update/hide rally flag for selected producer buildings.
pub fn rally_flag_system(
    mut commands: Commands,
    selected_buildings: Query<
        (&Owner, Option<&RallyPoint>),
        (With<Building>, With<Producer>, With<Selected>),
    >,
    mut flags: Query<(Entity, &mut Transform, &mut Visibility), With<RallyFlag>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Find the rally point of the first selected owned producer building
    let mut rally_target = None;
    for (owner, rally) in selected_buildings.iter() {
        if owner.player_id != LOCAL_PLAYER {
            continue;
        }
        if let Some(rp) = rally {
            rally_target = Some(rp.target);
            break;
        }
    }

    if let Some(target) = rally_target {
        let world = WorldPos::from_grid(target);
        let screen = world_to_screen(world);
        let z = depth_z(world) + 0.5; // slightly above ground

        if let Some((_, mut transform, mut visibility)) = flags.iter_mut().next() {
            // Update existing flag
            transform.translation = Vec3::new(screen.x, -screen.y, z);
            *visibility = Visibility::Inherited;
        } else {
            // Spawn new flag — small green diamond
            let mesh = meshes.add(Rhombus::new(TILE_HALF_HEIGHT * 0.6, TILE_HALF_HEIGHT * 0.6));
            let mat = materials.add(ColorMaterial::from_color(Color::srgba(0.2, 1.0, 0.4, 0.7)));

            commands.spawn((
                RallyFlag,
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(screen.x, -screen.y, z),
            ));
        }
    } else {
        // No rally to show — hide all flags
        for (_, _, mut visibility) in flags.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }
}
