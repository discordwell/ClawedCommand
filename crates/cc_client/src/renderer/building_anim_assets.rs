use bevy::prelude::*;

use crate::renderer::building_gen::{ALL_BUILDING_KINDS, building_slug};

/// Sprite sheet asset handles for building construction and ambient animations.
/// Each building has up to two sheets (construct, ambient) stored as texture atlas layouts.
/// If the sheet file doesn't exist on disk, the entry is `None` and the building
/// stays static (existing behavior).
#[derive(Resource)]
pub struct BuildingAnimSheets {
    /// Construction sheet handles: (image, layout) per building kind, indexed by building_kind_index.
    pub construct: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 48],
    /// Ambient idle sheet handles: (image, layout) per building kind, indexed by building_kind_index.
    pub ambient: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 48],
}

/// Frame size for building animation sheets: 1024x1024 pixels per frame, 4 frames per row.
const BUILDING_SHEET_FRAME_SIZE: UVec2 = UVec2::new(1024, 1024);
const BUILDING_SHEET_COLUMNS: u32 = 4;
const BUILDING_SHEET_ROWS: u32 = 1;

/// Load building animation sprite sheet assets at startup.
/// Checks for `assets/sprites/buildings/{slug}_{construct|ambient}.png` on disk.
/// Gracefully falls back to `None` if sheets don't exist.
pub fn load_building_anim_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let layout = TextureAtlasLayout::from_grid(
        BUILDING_SHEET_FRAME_SIZE,
        BUILDING_SHEET_COLUMNS,
        BUILDING_SHEET_ROWS,
        None,
        None,
    );
    let layout_handle = layouts.add(layout);

    let mut construct: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 48] =
        [const { None }; 48];
    let mut ambient: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 48] =
        [const { None }; 48];

    for (i, kind) in ALL_BUILDING_KINDS.iter().enumerate() {
        let slug = building_slug(*kind);

        // Construction sheet
        let construct_path = format!("sprites/buildings/{slug}_construct.png");
        if super::asset_exists_on_disk(&construct_path) {
            construct[i] = Some((asset_server.load(construct_path), layout_handle.clone()));
        }

        // Ambient sheet
        let ambient_path = format!("sprites/buildings/{slug}_ambient.png");
        if super::asset_exists_on_disk(&ambient_path) {
            ambient[i] = Some((asset_server.load(ambient_path), layout_handle.clone()));
        }
    }

    commands.insert_resource(BuildingAnimSheets { construct, ambient });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_sheet_frame_size_is_1024() {
        assert_eq!(BUILDING_SHEET_FRAME_SIZE, UVec2::new(1024, 1024));
    }

    #[test]
    fn building_sheet_layout_is_4x1() {
        assert_eq!(BUILDING_SHEET_COLUMNS, 4);
        assert_eq!(BUILDING_SHEET_ROWS, 1);
    }

    #[test]
    fn construct_paths_are_consistent() {
        for kind in ALL_BUILDING_KINDS {
            let slug = building_slug(kind);
            let path = format!("sprites/buildings/{slug}_construct.png");
            assert!(path.starts_with("sprites/buildings/"));
            assert!(path.ends_with("_construct.png"));
        }
    }

    #[test]
    fn ambient_paths_are_consistent() {
        for kind in ALL_BUILDING_KINDS {
            let slug = building_slug(kind);
            let path = format!("sprites/buildings/{slug}_ambient.png");
            assert!(path.starts_with("sprites/buildings/"));
            assert!(path.ends_with("_ambient.png"));
        }
    }
}
