use bevy::prelude::*;

use cc_core::components::UnitKind;

/// Resource holding unit sprite image handles (loaded from disk).
#[derive(Resource)]
pub struct UnitSprites {
    /// One image handle per UnitKind (indexed by kind_index).
    pub sprites: [Handle<Image>; 60],
}

/// Map UnitKind to array index.
pub fn kind_index(kind: UnitKind) -> usize {
    match kind {
        // Cat (catGPT) units: 0-9
        UnitKind::Pawdler => 0,
        UnitKind::Nuisance => 1,
        UnitKind::Chonk => 2,
        UnitKind::FlyingFox => 3,
        UnitKind::Hisser => 4,
        UnitKind::Yowler => 5,
        UnitKind::Mouser => 6,
        UnitKind::Catnapper => 7,
        UnitKind::FerretSapper => 8,
        UnitKind::MechCommander => 9,
        // Clawed (mice) units: 10-19
        UnitKind::Nibblet => 10,
        UnitKind::Swarmer => 11,
        UnitKind::Gnawer => 12,
        UnitKind::Shrieker => 13,
        UnitKind::Tunneler => 14,
        UnitKind::Sparks => 15,
        UnitKind::Quillback => 16,
        UnitKind::Whiskerwitch => 17,
        UnitKind::Plaguetail => 18,
        UnitKind::WarrenMarshal => 19,
        // Murder (corvids) units: 20-29
        UnitKind::MurderScrounger => 20,
        UnitKind::Sentinel => 21,
        UnitKind::Rookclaw => 22,
        UnitKind::Magpike => 23,
        UnitKind::Magpyre => 24,
        UnitKind::Jaycaller => 25,
        UnitKind::Jayflicker => 26,
        UnitKind::Dusktalon => 27,
        UnitKind::Hootseer => 28,
        UnitKind::CorvusRex => 29,
        // Seekers (badgers) units: 30-39
        UnitKind::Delver => 30,
        UnitKind::Ironhide => 31,
        UnitKind::Cragback => 32,
        UnitKind::Warden => 33,
        UnitKind::Sapjaw => 34,
        UnitKind::Wardenmother => 35,
        UnitKind::SeekerTunneler => 36,
        UnitKind::Embermaw => 37,
        UnitKind::Dustclaw => 38,
        UnitKind::Gutripper => 39,
        // Croak (axolotls) units: 40-49
        UnitKind::Ponderer => 40,
        UnitKind::Regeneron => 41,
        UnitKind::Broodmother => 42,
        UnitKind::Gulper => 43,
        UnitKind::Eftsaber => 44,
        UnitKind::Croaker => 45,
        UnitKind::Leapfrog => 46,
        UnitKind::Shellwarden => 47,
        UnitKind::Bogwhisper => 48,
        UnitKind::MurkCommander => 49,
        // LLAMA (raccoons) units: 50-59
        UnitKind::Scrounger => 50,
        UnitKind::Bandit => 51,
        UnitKind::HeapTitan => 52,
        UnitKind::GlitchRat => 53,
        UnitKind::PatchPossum => 54,
        UnitKind::GreaseMonkey => 55,
        UnitKind::DeadDropUnit => 56,
        UnitKind::Wrecker => 57,
        UnitKind::DumpsterDiver => 58,
        UnitKind::JunkyardKing => 59,
    }
}

/// Return the file name slug for a unit kind (e.g. "pawdler", "flying_fox").
pub fn unit_slug(kind: UnitKind) -> &'static str {
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
        UnitKind::Nibblet => "nibblet",
        UnitKind::Swarmer => "swarmer",
        UnitKind::Gnawer => "gnawer",
        UnitKind::Shrieker => "shrieker",
        UnitKind::Tunneler => "tunneler",
        UnitKind::Sparks => "sparks",
        UnitKind::Quillback => "quillback",
        UnitKind::Whiskerwitch => "whiskerwitch",
        UnitKind::Plaguetail => "plaguetail",
        UnitKind::WarrenMarshal => "warren_marshal",
        // Murder (corvids)
        UnitKind::MurderScrounger => "murder_scrounger",
        UnitKind::Sentinel => "sentinel",
        UnitKind::Rookclaw => "rookclaw",
        UnitKind::Magpike => "magpike",
        UnitKind::Magpyre => "magpyre",
        UnitKind::Jaycaller => "jaycaller",
        UnitKind::Jayflicker => "jayflicker",
        UnitKind::Dusktalon => "dusktalon",
        UnitKind::Hootseer => "hootseer",
        UnitKind::CorvusRex => "corvus_rex",
        // Seekers (badgers)
        UnitKind::Delver => "delver",
        UnitKind::Ironhide => "ironhide",
        UnitKind::Cragback => "cragback",
        UnitKind::Warden => "warden",
        UnitKind::Sapjaw => "sapjaw",
        UnitKind::Wardenmother => "wardenmother",
        UnitKind::SeekerTunneler => "seeker_tunneler",
        UnitKind::Embermaw => "embermaw",
        UnitKind::Dustclaw => "dustclaw",
        UnitKind::Gutripper => "gutripper",
        // Croak (axolotls)
        UnitKind::Ponderer => "ponderer",
        UnitKind::Regeneron => "regeneron",
        UnitKind::Broodmother => "broodmother",
        UnitKind::Gulper => "gulper",
        UnitKind::Eftsaber => "eftsaber",
        UnitKind::Croaker => "croaker",
        UnitKind::Leapfrog => "leapfrog",
        UnitKind::Shellwarden => "shellwarden",
        UnitKind::Bogwhisper => "bogwhisper",
        UnitKind::MurkCommander => "murk_commander",
        // LLAMA (raccoons)
        UnitKind::Scrounger => "scrounger",
        UnitKind::Bandit => "bandit",
        UnitKind::HeapTitan => "heap_titan",
        UnitKind::GlitchRat => "glitch_rat",
        UnitKind::PatchPossum => "patch_possum",
        UnitKind::GreaseMonkey => "grease_monkey",
        UnitKind::DeadDropUnit => "dead_drop_unit",
        UnitKind::Wrecker => "wrecker",
        UnitKind::DumpsterDiver => "dumpster_diver",
        UnitKind::JunkyardKing => "junkyard_king",
    }
}

/// Return the asset path for a unit's idle sprite PNG (relative to `assets/`).
pub fn sprite_file_path(kind: UnitKind) -> String {
    let name = unit_slug(kind);
    format!("sprites/units/{name}_idle.png")
}

/// All unit kinds in canonical order (cats 0-9, clawed 10-19, murder 20-29,
/// seekers 30-39, croak 40-49, llama 50-59).
pub const ALL_KINDS: [UnitKind; 60] = [
    // Cat (catGPT) units: 0-9
    UnitKind::Pawdler,
    UnitKind::Nuisance,
    UnitKind::Chonk,
    UnitKind::FlyingFox,
    UnitKind::Hisser,
    UnitKind::Yowler,
    UnitKind::Mouser,
    UnitKind::Catnapper,
    UnitKind::FerretSapper,
    UnitKind::MechCommander,
    // Clawed (mice) units: 10-19
    UnitKind::Nibblet,
    UnitKind::Swarmer,
    UnitKind::Gnawer,
    UnitKind::Shrieker,
    UnitKind::Tunneler,
    UnitKind::Sparks,
    UnitKind::Quillback,
    UnitKind::Whiskerwitch,
    UnitKind::Plaguetail,
    UnitKind::WarrenMarshal,
    // Murder (corvids) units: 20-29
    UnitKind::MurderScrounger,
    UnitKind::Sentinel,
    UnitKind::Rookclaw,
    UnitKind::Magpike,
    UnitKind::Magpyre,
    UnitKind::Jaycaller,
    UnitKind::Jayflicker,
    UnitKind::Dusktalon,
    UnitKind::Hootseer,
    UnitKind::CorvusRex,
    // Seekers (badgers) units: 30-39
    UnitKind::Delver,
    UnitKind::Ironhide,
    UnitKind::Cragback,
    UnitKind::Warden,
    UnitKind::Sapjaw,
    UnitKind::Wardenmother,
    UnitKind::SeekerTunneler,
    UnitKind::Embermaw,
    UnitKind::Dustclaw,
    UnitKind::Gutripper,
    // Croak (axolotls) units: 40-49
    UnitKind::Ponderer,
    UnitKind::Regeneron,
    UnitKind::Broodmother,
    UnitKind::Gulper,
    UnitKind::Eftsaber,
    UnitKind::Croaker,
    UnitKind::Leapfrog,
    UnitKind::Shellwarden,
    UnitKind::Bogwhisper,
    UnitKind::MurkCommander,
    // LLAMA (raccoons) units: 50-59
    UnitKind::Scrounger,
    UnitKind::Bandit,
    UnitKind::HeapTitan,
    UnitKind::GlitchRat,
    UnitKind::PatchPossum,
    UnitKind::GreaseMonkey,
    UnitKind::DeadDropUnit,
    UnitKind::Wrecker,
    UnitKind::DumpsterDiver,
    UnitKind::JunkyardKing,
];

/// Load all 60 unit sprite PNGs from disk at startup.
pub fn generate_unit_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tracker: ResMut<crate::loading::LoadingTracker>,
) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(60);

    for kind in ALL_KINDS {
        let asset_path = sprite_file_path(kind);
        let handle = asset_server.load(asset_path);
        tracker.track(&handle);
        handles.push(handle);
    }

    commands.insert_resource(UnitSprites {
        sprites: handles.try_into().expect("exactly 60 unit sprites"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_file_paths_match_catalog() {
        // Cat unit paths
        assert_eq!(
            sprite_file_path(UnitKind::Pawdler),
            "sprites/units/pawdler_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Nuisance),
            "sprites/units/nuisance_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Chonk),
            "sprites/units/chonk_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::FlyingFox),
            "sprites/units/flying_fox_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Hisser),
            "sprites/units/hisser_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Yowler),
            "sprites/units/yowler_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Mouser),
            "sprites/units/mouser_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Catnapper),
            "sprites/units/catnapper_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::FerretSapper),
            "sprites/units/ferret_sapper_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::MechCommander),
            "sprites/units/mech_commander_idle.png"
        );
        // Clawed (mice) unit paths
        assert_eq!(
            sprite_file_path(UnitKind::Nibblet),
            "sprites/units/nibblet_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Swarmer),
            "sprites/units/swarmer_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Gnawer),
            "sprites/units/gnawer_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Shrieker),
            "sprites/units/shrieker_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Tunneler),
            "sprites/units/tunneler_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Sparks),
            "sprites/units/sparks_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Quillback),
            "sprites/units/quillback_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Whiskerwitch),
            "sprites/units/whiskerwitch_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Plaguetail),
            "sprites/units/plaguetail_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::WarrenMarshal),
            "sprites/units/warren_marshal_idle.png"
        );
        // Murder (corvid) paths
        assert_eq!(
            sprite_file_path(UnitKind::MurderScrounger),
            "sprites/units/murder_scrounger_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::CorvusRex),
            "sprites/units/corvus_rex_idle.png"
        );
        // Seekers (badger) paths
        assert_eq!(
            sprite_file_path(UnitKind::Delver),
            "sprites/units/delver_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::Gutripper),
            "sprites/units/gutripper_idle.png"
        );
        // Croak (axolotl) paths
        assert_eq!(
            sprite_file_path(UnitKind::Ponderer),
            "sprites/units/ponderer_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::MurkCommander),
            "sprites/units/murk_commander_idle.png"
        );
        // LLAMA (raccoon) paths
        assert_eq!(
            sprite_file_path(UnitKind::Scrounger),
            "sprites/units/scrounger_idle.png"
        );
        assert_eq!(
            sprite_file_path(UnitKind::JunkyardKing),
            "sprites/units/junkyard_king_idle.png"
        );
    }

    #[test]
    fn all_kinds_have_sprite_paths() {
        for kind in ALL_KINDS {
            let path = sprite_file_path(kind);
            assert!(
                path.starts_with("sprites/units/"),
                "Path should be under sprites/units/: {path}"
            );
            assert!(
                path.ends_with("_idle.png"),
                "Path should end with _idle.png: {path}"
            );
        }
    }

    #[test]
    fn kind_index_covers_all_kinds() {
        for (i, kind) in ALL_KINDS.iter().enumerate() {
            assert_eq!(kind_index(*kind), i, "kind_index mismatch for {kind:?}");
        }
    }

    #[test]
    fn all_kinds_constant_has_sixty_entries() {
        assert_eq!(ALL_KINDS.len(), 60);
    }

    #[test]
    fn all_sprite_files_exist_on_disk() {
        let asset_root = std::path::Path::new("../../assets");
        for kind in ALL_KINDS {
            let asset_path = sprite_file_path(kind);
            let full_path = asset_root.join(&asset_path);
            assert!(
                full_path.exists(),
                "Sprite file missing for {kind:?}: {}",
                full_path.display()
            );
        }
    }

    #[test]
    fn non_cat_unit_slugs_are_valid() {
        // Check all 50 non-cat units (indices 10-59) have valid, unique slugs
        for kind in &ALL_KINDS[10..] {
            let slug = unit_slug(*kind);
            assert!(!slug.is_empty(), "Empty slug for {kind:?}");
            assert_ne!(
                slug, "pawdler",
                "Unit {kind:?} should not fall through to pawdler"
            );
        }
    }

    #[test]
    fn all_slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for kind in ALL_KINDS {
            let slug = unit_slug(kind);
            assert!(seen.insert(slug), "Duplicate slug: {slug} for {kind:?}");
        }
    }
}
