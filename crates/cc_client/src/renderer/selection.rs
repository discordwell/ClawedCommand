use bevy::prelude::*;

use crate::renderer::zoom_lod::ZoomTier;
use crate::setup::{BuildingMesh, TeamMaterials, UnitMesh, team_color};
use cc_core::components::{Dead, Owner, Selected};

/// Marker for selection ring child entity.
#[derive(Component)]
pub struct SelectionRing;

/// Update unit sprite tint based on ownership and selection state.
/// Spawn/despawn selection ring annulus on selection changes.
pub fn render_selection_indicators(
    mut commands: Commands,
    team_mats: Option<Res<TeamMaterials>>,
    tier: Res<ZoomTier>,
    mut meshes: ResMut<Assets<Mesh>>,
    // Units with Sprite (new procedural sprites)
    mut sprite_units: Query<
        (Entity, &mut Sprite, &Owner, Option<&Selected>, Option<&Children>),
        (With<UnitMesh>, Without<Dead>),
    >,
    ring_query: Query<Entity, With<SelectionRing>>,
    added_selected_units: Query<Entity, (With<UnitMesh>, Added<Selected>)>,
    added_selected_buildings: Query<Entity, (With<BuildingMesh>, Added<Selected>, Without<UnitMesh>)>,
    mut removed_selected: RemovedComponents<Selected>,
    all_with_children: Query<Option<&Children>, Or<(With<UnitMesh>, With<BuildingMesh>)>>,
) {
    let Some(team_mats) = team_mats else {
        return;
    };

    // In Strategic mode, hide unit sprites via alpha=0 so children (StrategicIcon) stay visible
    let sprite_alpha = if *tier == ZoomTier::Strategic { 0.0 } else { 1.0 };

    // Update sprite tint based on selection state
    for (_entity, mut sprite, owner, selected, _children) in sprite_units.iter_mut() {
        if selected.is_some() {
            sprite.color = Color::srgba(0.5, 0.9, 1.0, sprite_alpha);
        } else {
            sprite.color = team_color(owner.player_id).with_alpha(sprite_alpha);
        }
    }

    // Spawn selection rings for newly selected units and buildings
    let newly_selected: Vec<Entity> = added_selected_units
        .iter()
        .chain(added_selected_buildings.iter())
        .collect();

    if !newly_selected.is_empty() {
        let ring_mesh = meshes.add(Annulus::new(10.0, 12.0));
        let ring_mat = team_mats.selected.clone();
        for entity in newly_selected {
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

    // Despawn selection rings for deselected units and buildings
    for entity in removed_selected.read() {
        if let Ok(Some(children)) = all_with_children.get(entity) {
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
