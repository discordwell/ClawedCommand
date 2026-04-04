use bevy::prelude::*;

use cc_core::terrain::TerrainType;

/// Resource holding terrain sprite atlas handles.
/// Until real art is generated, falls back to colored rectangles.
#[derive(Resource, Default)]
pub struct TerrainAtlas {
    /// Base tile atlas (one sprite per terrain type). None = use colored rects.
    pub base_atlas: Option<Handle<TextureAtlasLayout>>,
    pub base_image: Option<Handle<Image>>,
    /// Transition overlay atlases per terrain type. None = skip overlays.
    pub transition_atlases: [Option<TransitionAtlasEntry>; 15],
    /// Whether real art is loaded (false = fallback to colored rects).
    pub art_loaded: bool,
}

#[derive(Clone)]
pub struct TransitionAtlasEntry {
    pub layout: Handle<TextureAtlasLayout>,
    pub image: Handle<Image>,
}

impl TerrainAtlas {
    /// Try to load terrain atlas assets. Returns whether any art was found.
    pub fn try_load(asset_server: &AssetServer, layouts: &mut Assets<TextureAtlasLayout>) -> Self {
        let mut atlas = Self::default();

        // Try loading base terrain tile sheet
        // Expected: assets/terrain/terrain_base_atlas.png (15 tiles in a row, 128x128 each)
        let base_path = "terrain/terrain_base_atlas.png";
        if asset_server.get_handle::<Image>(base_path).is_some() {
            let layout = TextureAtlasLayout::from_grid(UVec2::new(128, 128), 15, 1, None, None);
            let layout_handle = layouts.add(layout);
            atlas.base_atlas = Some(layout_handle);
            atlas.base_image = Some(asset_server.load(base_path));
            atlas.art_loaded = true;
        }

        // Try loading transition sheets per terrain type
        for terrain in TerrainType::ALL {
            let idx = terrain as usize;
            let path = format!("terrain/transition_{}.png", terrain_name(terrain));
            if asset_server.get_handle::<Image>(&path).is_some() {
                let layout = TextureAtlasLayout::from_grid(UVec2::new(128, 128), 4, 3, None, None);
                let layout_handle = layouts.add(layout);
                atlas.transition_atlases[idx] = Some(TransitionAtlasEntry {
                    layout: layout_handle,
                    image: asset_server.load(&path),
                });
            }
        }

        atlas
    }
}

/// Map terrain type to filename component.
pub fn terrain_name(terrain: TerrainType) -> &'static str {
    match terrain {
        TerrainType::Grass => "grass",
        TerrainType::Dirt => "dirt",
        TerrainType::Sand => "sand",
        TerrainType::Forest => "forest",
        TerrainType::Water => "water",
        TerrainType::Shallows => "shallows",
        TerrainType::Rock => "rock",
        TerrainType::Ramp => "ramp",
        TerrainType::Road => "road",
        TerrainType::TechRuins => "tech_ruins",
        TerrainType::Concrete => "concrete",
        TerrainType::Linoleum => "linoleum",
        TerrainType::CarpetTile => "carpet_tile",
        TerrainType::MetalGrate => "metal_grate",
        TerrainType::DryWall => "drywall",
    }
}
