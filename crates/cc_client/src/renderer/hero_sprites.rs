use std::collections::HashMap;

use bevy::prelude::*;

use cc_core::hero::{ALL_HEROES, HeroId, hero_slug};

/// Resource holding hero-specific sprite image handles.
/// Heroes without a sprite file on disk fall back to their base_kind sprite.
#[derive(Resource, Default)]
pub struct HeroSprites {
    pub sprites: HashMap<HeroId, Handle<Image>>,
}

/// Resource holding hero-specific walk animation sheets.
#[derive(Resource, Default)]
pub struct HeroAnimSheets {
    pub walk: HashMap<HeroId, (Handle<Image>, Handle<TextureAtlasLayout>)>,
}

/// Load hero sprites + walk sheets at startup.
pub fn load_hero_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut sprites = HashMap::new();
    let mut walk_sheets = HashMap::new();

    let walk_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(128, 128),
        4,
        1,
        None,
        None,
    ));

    for hero in ALL_HEROES {
        let slug = hero_slug(hero);

        // Idle sprite
        let idle_path = format!("sprites/heroes/{slug}_idle.png");
        if super::asset_exists_on_disk(&idle_path) {
            sprites.insert(hero, asset_server.load(idle_path));
        }

        // Walk sheet
        let walk_path = format!("sprites/heroes/{slug}_walk.png");
        if super::asset_exists_on_disk(&walk_path) {
            walk_sheets.insert(hero, (asset_server.load(walk_path), walk_layout.clone()));
        }
    }

    commands.insert_resource(HeroSprites { sprites });
    commands.insert_resource(HeroAnimSheets { walk: walk_sheets });
}
