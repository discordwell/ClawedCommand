use bevy::prelude::*;

use super::tile_gen::ProceduralTiles;
use super::tilemap::WaterTile;
use cc_core::terrain::TerrainType;

/// Animate water tiles by swapping between primary and alt tile images.
pub fn animate_water(
    time: Res<Time>,
    tiles: Option<Res<ProceduralTiles>>,
    mut query: Query<(&mut WaterTile, &mut Sprite)>,
) {
    let Some(tiles) = tiles else {
        return;
    };

    let t = time.elapsed_secs();
    // Swap every ~1.5 seconds
    let show_alt = ((t * 0.67) as u32) % 2 == 1;

    for (mut water, mut sprite) in query.iter_mut() {
        if water.showing_alt != show_alt {
            water.showing_alt = show_alt;
            if water.is_shallows {
                sprite.image = if show_alt {
                    tiles.shallows_alt.clone()
                } else {
                    tiles.terrain[TerrainType::Shallows as usize].clone()
                };
            } else {
                sprite.image = if show_alt {
                    tiles.water_alt.clone()
                } else {
                    tiles.terrain[TerrainType::Water as usize].clone()
                };
            }
        }
    }
}
