use std::collections::HashMap;

use bevy::prelude::*;

use cc_core::hero::{HeroId, ALL_HEROES, hero_slug};

/// Resource holding hero-specific sprite image handles.
/// Heroes without a sprite file on disk fall back to their base_kind sprite.
#[derive(Resource, Default)]
pub struct HeroSprites {
    pub sprites: HashMap<HeroId, Handle<Image>>,
}

/// Load hero sprites at startup by scanning `assets/sprites/heroes/{slug}_idle.png`.
/// Only loads sprites that exist on disk.
pub fn load_hero_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut sprites = HashMap::new();
    for hero in ALL_HEROES {
        let path = format!("sprites/heroes/{}_idle.png", hero_slug(hero));
        if super::asset_exists_on_disk(&path) {
            sprites.insert(hero, asset_server.load(path));
        }
    }
    commands.insert_resource(HeroSprites { sprites });
}
