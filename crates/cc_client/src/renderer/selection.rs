use bevy::prelude::*;

use crate::setup::{TeamMaterials, UnitMesh, team_color};
use cc_core::components::{Owner, Selected};

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: u8 = 0;

/// Marker for selection ring child entity.
#[derive(Component)]
pub struct SelectionRing;

/// Update unit sprite tint based on ownership and selection state.
/// Spawn/despawn selection ring annulus on selection changes.
pub fn render_selection_indicators(
    mut commands: Commands,
    team_mats: Option<Res<TeamMaterials>>,
    mut meshes: ResMut<Assets<Mesh>>,
    // Units with Sprite (new procedural sprites)
    mut sprite_units: Query<
        (Entity, &mut Sprite, &Owner, Option<&Selected>, Option<&Children>),
        With<UnitMesh>,
    >,
    ring_query: Query<Entity, With<SelectionRing>>,
    added_selected: Query<Entity, (With<UnitMesh>, Added<Selected>)>,
    mut removed_selected: RemovedComponents<Selected>,
) {
    let Some(team_mats) = team_mats else {
        return;
    };

    // Update sprite tint based on selection state
    for (_entity, mut sprite, owner, selected, _children) in sprite_units.iter_mut() {
        if selected.is_some() {
            sprite.color = Color::srgb(0.5, 0.9, 1.0); // Bright cyan tint
        } else {
            sprite.color = team_color(owner.player_id);
        }
    }

    // Spawn selection rings for newly selected units
    if !added_selected.is_empty() {
        let ring_mesh = meshes.add(Annulus::new(10.0, 12.0));
        let ring_mat = team_mats.selected.clone();
        for entity in added_selected.iter() {
            let ring = commands
                .spawn((
                    SelectionRing,
                    Mesh2d(ring_mesh.clone()),
                    MeshMaterial2d(ring_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.05),
                ))
                .id();
            commands.entity(entity).add_children(&[ring]);
        }
    }

    // Despawn selection rings for deselected units
    for entity in removed_selected.read() {
        if let Ok((_e, _sprite, _owner, _sel, Some(children))) = sprite_units.get(entity) {
            for child in children.iter() {
                if ring_query.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }
    }
}

/// Pulse selection ring scale using sin(time).
pub fn pulse_selection_rings(time: Res<Time>, mut rings: Query<&mut Transform, With<SelectionRing>>) {
    let t = time.elapsed_secs();
    let pulse = 1.0 + (t * 3.0).sin() * 0.1;
    for mut transform in rings.iter_mut() {
        transform.scale = Vec3::splat(pulse);
    }
}
