use bevy::prelude::*;

use cc_core::components::UnitKind;

use crate::renderer::unit_gen::ALL_KINDS;

/// Sprite sheet asset handles for walk and attack animations.
/// Each unit has up to two sheets (walk, attack) stored as texture atlas layouts.
/// If the sheet file doesn't exist on disk, the entry is `None` and the unit
/// uses its idle frame instead.
#[derive(Resource)]
pub struct AnimSheets {
    /// Walk sheet handles: (image, layout) per unit kind, indexed by kind_index.
    pub walk: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 10],
    /// Attack sheet handles: (image, layout) per unit kind, indexed by kind_index.
    pub attack: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 10],
}

/// Frame size for sprite sheets: 128x128 pixels per frame, 4 frames per row.
const SHEET_FRAME_SIZE: UVec2 = UVec2::new(128, 128);
const SHEET_COLUMNS: u32 = 4;
const SHEET_ROWS: u32 = 1;

/// Return the unit name slug used in file paths.
fn unit_slug(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Pawdler => "pawdler",
        UnitKind::Nuisance => "nuisance",
        UnitKind::Chonk => "chonk",
        UnitKind::FlyingFox => "flying_fox",
        UnitKind::Hisser => "hisser",
        UnitKind::Yowler => "yowler",
        UnitKind::Mouser => "mouser",
        UnitKind::Catnapper => "catnapper",
        UnitKind::FerretSapper => "ferret_sapper",
        UnitKind::MechCommander => "mech_commander",
        _ => "pawdler",
    }
}

/// Load animation sprite sheet assets at startup.
/// Checks for `assets/sprites/units/{slug}_{walk|attack}.png` on disk.
/// Gracefully falls back to `None` if sheets don't exist.
pub fn load_anim_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let layout = TextureAtlasLayout::from_grid(SHEET_FRAME_SIZE, SHEET_COLUMNS, SHEET_ROWS, None, None);
    let layout_handle = layouts.add(layout);

    let mut walk: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 10] = Default::default();
    let mut attack: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 10] = Default::default();

    for (i, kind) in ALL_KINDS.iter().enumerate() {
        let slug = unit_slug(*kind);

        // Walk sheet
        let walk_path = format!("sprites/units/{slug}_walk.png");
        #[cfg(not(target_arch = "wasm32"))]
        let walk_exists = std::path::Path::new("assets").join(&walk_path).exists();
        #[cfg(target_arch = "wasm32")]
        let walk_exists = false;

        if walk_exists {
            walk[i] = Some((asset_server.load(walk_path), layout_handle.clone()));
        }

        // Attack sheet
        let attack_path = format!("sprites/units/{slug}_attack.png");
        #[cfg(not(target_arch = "wasm32"))]
        let attack_exists = std::path::Path::new("assets").join(&attack_path).exists();
        #[cfg(target_arch = "wasm32")]
        let attack_exists = false;

        if attack_exists {
            attack[i] = Some((asset_server.load(attack_path), layout_handle.clone()));
        }
    }

    commands.insert_resource(AnimSheets { walk, attack });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_slugs_all_valid() {
        for kind in ALL_KINDS {
            let slug = unit_slug(kind);
            assert!(!slug.is_empty(), "Slug for {kind:?} should not be empty");
            assert!(
                slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "Slug for {kind:?} has invalid chars: {slug}"
            );
        }
    }

    #[test]
    fn sheet_frame_size_is_128() {
        assert_eq!(SHEET_FRAME_SIZE, UVec2::new(128, 128));
    }

    #[test]
    fn sheet_layout_is_4x1() {
        assert_eq!(SHEET_COLUMNS, 4);
        assert_eq!(SHEET_ROWS, 1);
    }
}
