use bevy::prelude::*;

use crate::setup::UnitMesh;
use cc_core::components::Dead;

/// When a unit is first marked Dead, clone its shared material into a unique handle
/// so the fade doesn't affect all living units sharing that material.
pub fn isolate_dead_material(
    mut commands: Commands,
    query: Query<(Entity, &MeshMaterial2d<ColorMaterial>), (Added<Dead>, With<UnitMesh>)>,
    materials: Res<Assets<ColorMaterial>>,
    mut material_assets: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, mat) in query.iter() {
        if let Some(existing) = materials.get(&mat.0) {
            let cloned = material_assets.add(existing.clone());
            commands.entity(entity).insert(MeshMaterial2d(cloned));
        }
    }
}

/// Fade out the now-unique material alpha on entities marked Dead.
pub fn death_fade_system(
    query: Query<&MeshMaterial2d<ColorMaterial>, (With<Dead>, With<UnitMesh>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for mat_handle in query.iter() {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let a = mat.color.alpha();
            mat.color.set_alpha((a - 0.15).max(0.0));
        }
    }
}
