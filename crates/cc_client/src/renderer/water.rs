use bevy::prelude::*;

use super::tilemap::WaterMaterials;

/// Oscillate the blue channel of shared water/shallows materials.
/// Only 4 material mutations per frame regardless of water tile count.
pub fn animate_water(
    time: Res<Time>,
    water_mats: Option<Res<WaterMaterials>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(water_mats) = water_mats else {
        return;
    };

    let t = time.elapsed_secs();
    let wave = (t * 1.5).sin() * 0.05;

    // Water base colors
    let water_base_a = Color::srgb(0.15, 0.35, 0.65 + wave);
    let water_base_b = Color::srgb(0.18, 0.38, 0.68 + wave);
    let shallows_base_a = Color::srgb(0.40, 0.68, 0.88 + wave);
    let shallows_base_b = Color::srgb(0.42, 0.70, 0.90 + wave);

    if let Some(mat) = materials.get_mut(&water_mats.water_a) {
        mat.color = water_base_a;
    }
    if let Some(mat) = materials.get_mut(&water_mats.water_b) {
        mat.color = water_base_b;
    }
    if let Some(mat) = materials.get_mut(&water_mats.shallows_a) {
        mat.color = shallows_base_a;
    }
    if let Some(mat) = materials.get_mut(&water_mats.shallows_b) {
        mat.color = shallows_base_b;
    }
}
