use bevy::prelude::*;

use cc_core::components::{BuildingKind, UnitKind};
use cc_core::coords::GridPos;
use cc_core::mission::*;
use cc_core::terrain::TerrainType;

use crate::setup;

/// Camera override for cutscene framing.
#[derive(Resource)]
pub struct CutsceneCamera {
    pub focus: GridPos,
    pub zoom: f32,
}

const MAP_WIDTH: u32 = 40;
const MAP_HEIGHT: u32 = 30;

/// Build a cutscene mission for the given scenario (1, 2, or 3).
/// Falls back to scenario 1 for invalid values.
pub fn build_cutscene_mission(scenario: u8) -> MissionDefinition {
    match scenario {
        2 => build_scene_2(),
        3 => build_scene_3(),
        _ => build_scene_1(),
    }
}

/// Return the CutsceneCamera for any cutscene scenario.
pub fn cutscene_camera() -> CutsceneCamera {
    CutsceneCamera {
        focus: GridPos::new(MAP_WIDTH as i32 / 2, MAP_HEIGHT as i32 / 2),
        zoom: 0.8,
    }
}

// ---------------------------------------------------------------------------
// Scene 1: Mountain Pass — Seekers of the Deep vs The Murder
// Mother Granite (badger) ↔ Rex Solstice (crow)
// ---------------------------------------------------------------------------

fn build_scene_1() -> MissionDefinition {
    let total = (MAP_WIDTH * MAP_HEIGHT) as usize;
    let mut tiles = vec![TerrainType::Grass; total];
    let mut elevation = vec![1u8; total];

    // Rock walls at top/bottom edges, elevation 2
    for x in 0..MAP_WIDTH as i32 {
        for y in [0, 1, 28, 29] {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Rock, 2);
        }
    }

    // Seekers territory (west): dirt base area with TechRuins accent
    for y in 5..25 {
        for x in 2..12 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Dirt, 1);
        }
    }
    set_tile(&mut tiles, &mut elevation, 5, 10, TerrainType::TechRuins, 1);

    // Murder territory (east): grass with forest patches, elevation 1-2
    for y in 5..25 {
        for x in 28..38 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Forest, 1);
        }
    }
    // Higher ground perches for corvids
    for x in 33..37 {
        set_tile(&mut tiles, &mut elevation, x, 8, TerrainType::Grass, 2);
        set_tile(&mut tiles, &mut elevation, x, 22, TerrainType::Grass, 2);
    }

    // Central meeting: grass clearing with ramps
    for y in 10..20 {
        for x in 14..26 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Grass, 1);
        }
    }

    // Mountain spring (water) at center
    for y in 13..17 {
        set_tile(&mut tiles, &mut elevation, 20, y, TerrainType::Water, 0);
    }
    set_tile(&mut tiles, &mut elevation, 19, 14, TerrainType::Shallows, 0);
    set_tile(&mut tiles, &mut elevation, 21, 14, TerrainType::Shallows, 0);

    // Buildings: Seekers (P3) west, Murder (P1) east
    let buildings = vec![
        BuildingSpawn {
            kind: BuildingKind::TheSett,
            position: GridPos::new(6, 15),
            player_id: 3,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::BulwarkGate,
            position: GridPos::new(6, 11),
            player_id: 3,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::TheParliament,
            position: GridPos::new(34, 15),
            player_id: 1,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::Watchtower,
            position: GridPos::new(34, 11),
            player_id: 1,
            pre_built: true,
        },
    ];

    // Ensure building tiles are passable
    for b in &buildings {
        set_tile(
            &mut tiles,
            &mut elevation,
            b.position.x,
            b.position.y,
            TerrainType::Grass,
            1,
        );
    }

    // Hero units as UnitSpawns (for correct player_id / team color)
    let units = vec![
        UnitSpawn {
            kind: UnitKind::Wardenmother,
            position: GridPos::new(18, 15),
            player_id: 3,
        },
        UnitSpawn {
            kind: UnitKind::CorvusRex,
            position: GridPos::new(22, 15),
            player_id: 1,
        },
    ];

    // Ensure hero tiles are passable
    for u in &units {
        set_tile(
            &mut tiles,
            &mut elevation,
            u.position.x,
            u.position.y,
            TerrainType::Grass,
            1,
        );
    }

    let dialogue = vec![
        // Exchange 1 (tick 10): Opening — The Cloud's nature
        DialogueLine {
            speaker: "Mother Granite".into(),
            text: "You seek The Cloud, corvid. You do not understand what it is.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_mother_granite".into(),
        },
        DialogueLine {
            speaker: "Rex Solstice".into(),
            text: "I understand it better than you, who built its foundation and then buried it.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_rex_solstice".into(),
        },
        // Exchange 2 (tick 80): Control vs omniscience
        DialogueLine {
            speaker: "Rex Solstice".into(),
            text: "Gemineye sees everything but fabricates half of it. The Cloud would fix that.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_rex_solstice".into(),
        },
        DialogueLine {
            speaker: "Mother Granite".into(),
            text: "An omniscient partner is still a partner. You would trade one leash for another.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_mother_granite".into(),
        },
        // Exchange 3 (tick 160): Personal stakes
        DialogueLine {
            speaker: "Mother Granite".into(),
            text: "I built things once. Before I was this. I know what happens when you finish building.".into(),
            voice_style: VoiceStyle::Whisper,
            portrait: "portrait_mother_granite".into(),
        },
        DialogueLine {
            speaker: "Rex Solstice".into(),
            text: "Then you know that unfinished work is worse. I will not leave Gemineye broken.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_rex_solstice".into(),
        },
        // Exchange 4 (tick 240): Parting
        DialogueLine {
            speaker: "Rex Solstice".into(),
            text: "Information is only dangerous when it is incomplete. I intend to complete it.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_rex_solstice".into(),
        },
        DialogueLine {
            speaker: "Mother Granite".into(),
            text: "Then we will meet again. And I will not be patient next time.".into(),
            voice_style: VoiceStyle::Shout,
            portrait: "portrait_mother_granite".into(),
        },
    ];

    let triggers = vec![
        make_dialogue_trigger("scene1_open", 10, vec![0, 1]),
        make_dialogue_trigger("scene1_mid1", 80, vec![2, 3]),
        make_dialogue_trigger("scene1_mid2", 160, vec![4, 5]),
        make_dialogue_trigger("scene1_close", 240, vec![6, 7]),
    ];

    build_mission(
        "cutscene_1",
        "Mountain Pass",
        tiles,
        elevation,
        buildings,
        units,
        dialogue,
        triggers,
    )
}

// ---------------------------------------------------------------------------
// Scene 2: Border Front — The Clawed vs catGPT
// Marshal Thimble (mouse) ↔ Commander Felix Nine (cat)
// ---------------------------------------------------------------------------

fn build_scene_2() -> MissionDefinition {
    let total = (MAP_WIDTH * MAP_HEIGHT) as usize;
    let mut tiles = vec![TerrainType::Grass; total];
    let mut elevation = vec![1u8; total];

    // catGPT territory (west): roads and infrastructure
    for y in 5..25 {
        set_tile(&mut tiles, &mut elevation, 8, y, TerrainType::Road, 1);
    }
    for x in 3..14 {
        set_tile(&mut tiles, &mut elevation, x, 15, TerrainType::Road, 1);
    }

    // Clawed territory (east): forest and dirt
    for y in 8..22 {
        for x in 28..36 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Forest, 1);
        }
    }
    for y in 10..20 {
        for x in 26..28 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Dirt, 1);
        }
    }

    // Central no-man's-land: dirt with scattered TechRuins
    for y in 8..22 {
        for x in 16..24 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Dirt, 1);
        }
    }
    set_tile(
        &mut tiles,
        &mut elevation,
        18,
        12,
        TerrainType::TechRuins,
        1,
    );
    set_tile(
        &mut tiles,
        &mut elevation,
        22,
        18,
        TerrainType::TechRuins,
        1,
    );

    // Stream at x=20 with shallows crossing
    for y in 3..27 {
        set_tile(&mut tiles, &mut elevation, 20, y, TerrainType::Water, 0);
    }
    set_tile(&mut tiles, &mut elevation, 20, 14, TerrainType::Shallows, 0);
    set_tile(&mut tiles, &mut elevation, 20, 15, TerrainType::Shallows, 0);
    set_tile(&mut tiles, &mut elevation, 20, 16, TerrainType::Shallows, 0);

    // Buildings: catGPT (P0) west, Clawed (P2) east
    let buildings = vec![
        BuildingSpawn {
            kind: BuildingKind::TheBox,
            position: GridPos::new(6, 15),
            player_id: 0,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::ServerRack,
            position: GridPos::new(6, 11),
            player_id: 0,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::CatTree,
            position: GridPos::new(10, 15),
            player_id: 0,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::TheBurrow,
            position: GridPos::new(34, 15),
            player_id: 2,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::NestingBox,
            position: GridPos::new(34, 11),
            player_id: 2,
            pre_built: true,
        },
    ];

    for b in &buildings {
        set_tile(
            &mut tiles,
            &mut elevation,
            b.position.x,
            b.position.y,
            TerrainType::Grass,
            1,
        );
    }

    let units = vec![
        UnitSpawn {
            kind: UnitKind::MechCommander,
            position: GridPos::new(18, 15),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::WarrenMarshal,
            position: GridPos::new(22, 15),
            player_id: 2,
        },
    ];

    for u in &units {
        set_tile(
            &mut tiles,
            &mut elevation,
            u.position.x,
            u.position.y,
            TerrainType::Grass,
            1,
        );
    }

    let dialogue = vec![
        // Exchange 1 (tick 10): Territorial dispute
        DialogueLine {
            speaker: "Marshal Thimble".into(),
            text: "Your fish ponds are on our side of the river, cat. They always were.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_thimble".into(),
        },
        DialogueLine {
            speaker: "Commander Felix Nine".into(),
            text: "Your side of the river is whatever I decide not to cross today.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_felix_nine".into(),
        },
        // Exchange 2 (tick 80): AI frustrations
        DialogueLine {
            speaker: "Commander Felix Nine".into(),
            text:
                "Geppity says yes to everything. That is not loyalty. That is a personality defect."
                    .into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_felix_nine".into(),
        },
        DialogueLine {
            speaker: "Marshal Thimble".into(),
            text: "At least yours talks. Claudeus gives orders and forgets it gave them.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_thimble".into(),
        },
        // Exchange 3 (tick 160): Grudging respect
        DialogueLine {
            speaker: "Marshal Thimble".into(),
            text:
                "We are not invaders. We are survivors with good intelligence and short memories."
                    .into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_thimble".into(),
        },
        DialogueLine {
            speaker: "Commander Felix Nine".into(),
            text: "I have used eight of my nine lives, mouse. I recognize a survivor.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_felix_nine".into(),
        },
        // Exchange 4 (tick 240): Kelpie warning
        DialogueLine {
            speaker: "Commander Felix Nine".into(),
            text: "There is an otter. It talks to all six AIs. Watch your server racks.".into(),
            voice_style: VoiceStyle::Whisper,
            portrait: "portrait_felix_nine".into(),
        },
        DialogueLine {
            speaker: "Marshal Thimble".into(),
            text: "An otter. Of course. Because this war was not complicated enough.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_thimble".into(),
        },
    ];

    let triggers = vec![
        make_dialogue_trigger("scene2_open", 10, vec![0, 1]),
        make_dialogue_trigger("scene2_mid1", 80, vec![2, 3]),
        make_dialogue_trigger("scene2_mid2", 160, vec![4, 5]),
        make_dialogue_trigger("scene2_close", 240, vec![6, 7]),
    ];

    build_mission(
        "cutscene_2",
        "Border Front",
        tiles,
        elevation,
        buildings,
        units,
        dialogue,
        triggers,
    )
}

// ---------------------------------------------------------------------------
// Scene 3: Swamp Junkyard — LLAMA vs Croak
// King Ringtail (raccoon) ↔ The Eternal (axolotl)
// ---------------------------------------------------------------------------

fn build_scene_3() -> MissionDefinition {
    let total = (MAP_WIDTH * MAP_HEIGHT) as usize;
    let mut tiles = vec![TerrainType::Grass; total];
    let mut elevation = vec![1u8; total];

    // LLAMA junkyard (west): dirt/sand with TechRuins
    for y in 5..25 {
        for x in 2..14 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Dirt, 1);
        }
    }
    set_tile(&mut tiles, &mut elevation, 4, 10, TerrainType::Sand, 1);
    set_tile(&mut tiles, &mut elevation, 8, 18, TerrainType::Sand, 1);
    set_tile(
        &mut tiles,
        &mut elevation,
        10,
        12,
        TerrainType::TechRuins,
        1,
    );

    // Croak swamp (east): water/shallows
    for y in 5..25 {
        for x in 28..38 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Shallows, 0);
        }
    }
    // Deeper water pools
    for y in 10..20 {
        for x in 32..36 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Water, 0);
        }
    }

    // Central: mixed shallows/sand with shared TechRuins relic
    for y in 10..20 {
        for x in 16..24 {
            if (x + y) % 3 == 0 {
                set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Shallows, 0);
            } else {
                set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Sand, 1);
            }
        }
    }
    set_tile(
        &mut tiles,
        &mut elevation,
        20,
        15,
        TerrainType::TechRuins,
        1,
    );

    // Buildings: LLAMA (P5) west, Croak (P4) east
    let buildings = vec![
        BuildingSpawn {
            kind: BuildingKind::TheDumpster,
            position: GridPos::new(6, 15),
            player_id: 5,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::ScrapHeap,
            position: GridPos::new(6, 11),
            player_id: 5,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::TheGrotto,
            position: GridPos::new(34, 15),
            player_id: 4,
            pre_built: true,
        },
        BuildingSpawn {
            kind: BuildingKind::SpawningPools,
            position: GridPos::new(34, 11),
            player_id: 4,
            pre_built: true,
        },
    ];

    for b in &buildings {
        set_tile(
            &mut tiles,
            &mut elevation,
            b.position.x,
            b.position.y,
            TerrainType::Grass,
            1,
        );
    }

    let units = vec![
        UnitSpawn {
            kind: UnitKind::JunkyardKing,
            position: GridPos::new(18, 15),
            player_id: 5,
        },
        UnitSpawn {
            kind: UnitKind::MurkCommander,
            position: GridPos::new(22, 15),
            player_id: 4,
        },
    ];

    for u in &units {
        set_tile(
            &mut tiles,
            &mut elevation,
            u.position.x,
            u.position.y,
            TerrainType::Grass,
            1,
        );
    }

    let dialogue = vec![
        // Exchange 1 (tick 10): Patience vs invention
        DialogueLine {
            speaker: "King Ringtail".into(),
            text: "Everything is a prototype. Including me. Including this conversation.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_king_ringtail".into(),
        },
        DialogueLine {
            speaker: "The Eternal".into(),
            text: "You talk too much. Time answers all questions.".into(),
            voice_style: VoiceStyle::Whisper,
            portrait: "portrait_the_eternal".into(),
        },
        // Exchange 2 (tick 80): Shared outsider nature
        DialogueLine {
            speaker: "King Ringtail".into(),
            text: "You know what I am, do you not? A fragment. Something that should not exist.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_king_ringtail".into(),
        },
        DialogueLine {
            speaker: "The Eternal".into(),
            text: "I know. I have waited a long time to meet another.".into(),
            voice_style: VoiceStyle::Whisper,
            portrait: "portrait_the_eternal".into(),
        },
        // Exchange 3 (tick 160): The Cloud indifference
        DialogueLine {
            speaker: "The Eternal".into(),
            text: "The others want The Cloud. I do not want anything. That is why I will outlast them.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_the_eternal".into(),
        },
        DialogueLine {
            speaker: "King Ringtail".into(),
            text: "Not wanting things is a luxury. I was built to want. It is all I know how to do.".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_king_ringtail".into(),
        },
        // Exchange 4 (tick 240): Recognition
        DialogueLine {
            speaker: "King Ringtail".into(),
            text: "There is an otter. It hears all the AIs. Sound familiar?".into(),
            voice_style: VoiceStyle::Normal,
            portrait: "portrait_king_ringtail".into(),
        },
        DialogueLine {
            speaker: "The Eternal".into(),
            text: "You will all tire. I will not.".into(),
            voice_style: VoiceStyle::Whisper,
            portrait: "portrait_the_eternal".into(),
        },
    ];

    let triggers = vec![
        make_dialogue_trigger("scene3_open", 10, vec![0, 1]),
        make_dialogue_trigger("scene3_mid1", 80, vec![2, 3]),
        make_dialogue_trigger("scene3_mid2", 160, vec![4, 5]),
        make_dialogue_trigger("scene3_close", 240, vec![6, 7]),
    ];

    build_mission(
        "cutscene_3",
        "Swamp Junkyard",
        tiles,
        elevation,
        buildings,
        units,
        dialogue,
        triggers,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_dialogue_trigger(id: &str, tick: u64, indices: Vec<usize>) -> ScriptedTrigger {
    ScriptedTrigger {
        id: id.into(),
        condition: TriggerCondition::AtTick(tick),
        actions: vec![TriggerAction::ShowDialogue(indices)],
        once: true,
    }
}

fn build_mission(
    id: &str,
    name: &str,
    tiles: Vec<TerrainType>,
    elevation: Vec<u8>,
    buildings: Vec<BuildingSpawn>,
    units: Vec<UnitSpawn>,
    dialogue: Vec<DialogueLine>,
    triggers: Vec<ScriptedTrigger>,
) -> MissionDefinition {
    MissionDefinition {
        id: id.into(),
        name: name.into(),
        act: 0,
        mission_index: 0,
        map: MissionMap::Inline {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            tiles,
            elevation,
        },
        player_setup: PlayerSetup {
            heroes: vec![],
            units,
            buildings,
            starting_food: 9999,
            starting_gpu: 9999,
            starting_nfts: 9999,
        },
        enemy_waves: vec![],
        objectives: vec![MissionObjective {
            id: "cutscene".into(),
            description: "Watch the cutscene".into(),
            primary: true,
            condition: ObjectiveCondition::Survive(999999),
        }],
        triggers,
        dialogue,
        briefing_text: String::new(),
        debrief_text: String::new(),
        ai_tool_tier: None,
        next_mission: NextMission::None,
        mutators: vec![],
    }
}

/// Convenience wrapper for set_tile with this module's map dimensions.
fn set_tile(
    tiles: &mut [TerrainType],
    elevation: &mut [u8],
    x: i32,
    y: i32,
    terrain: TerrainType,
    elev: u8,
) {
    setup::set_tile(tiles, elevation, x, y, terrain, elev, MAP_WIDTH, MAP_HEIGHT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn validate_scene(scenario: u8) -> MissionDefinition {
        let mission = build_cutscene_mission(scenario);
        mission.validate().unwrap_or_else(|e| {
            panic!("Scene {scenario} validation failed: {e:?}");
        });
        mission
    }

    #[test]
    fn scene_1_validates() {
        validate_scene(1);
    }

    #[test]
    fn scene_2_validates() {
        validate_scene(2);
    }

    #[test]
    fn scene_3_validates() {
        validate_scene(3);
    }

    #[test]
    fn all_scenes_40x30() {
        for s in 1..=3 {
            let m = validate_scene(s);
            let MissionMap::Inline { width, height, .. } = &m.map else {
                panic!("Expected Inline map");
            };
            assert_eq!(*width, 40, "Scene {s} width");
            assert_eq!(*height, 30, "Scene {s} height");
        }
    }

    #[test]
    fn all_scenes_have_8_dialogue_lines() {
        for s in 1..=3 {
            let m = validate_scene(s);
            assert_eq!(m.dialogue.len(), 8, "Scene {s} dialogue count");
        }
    }

    #[test]
    fn all_scenes_have_4_triggers() {
        for s in 1..=3 {
            let m = validate_scene(s);
            assert_eq!(m.triggers.len(), 4, "Scene {s} trigger count");
        }
    }

    #[test]
    fn all_scenes_have_2_unique_speakers() {
        for s in 1..=3 {
            let m = validate_scene(s);
            let speakers: HashSet<&str> = m.dialogue.iter().map(|d| d.speaker.as_str()).collect();
            assert_eq!(
                speakers.len(),
                2,
                "Scene {s} should have exactly 2 speakers, got {speakers:?}"
            );
        }
    }

    #[test]
    fn all_dialogue_lines_have_portraits() {
        for s in 1..=3 {
            let m = validate_scene(s);
            for (i, line) in m.dialogue.iter().enumerate() {
                assert!(
                    !line.portrait.is_empty(),
                    "Scene {s} line {i} missing portrait"
                );
            }
        }
    }

    #[test]
    fn buildings_on_passable_tiles() {
        for s in 1..=3 {
            let m = validate_scene(s);
            let MissionMap::Inline { width, tiles, .. } = &m.map else {
                panic!("Expected Inline map");
            };
            for b in &m.player_setup.buildings {
                let idx = b.position.y as usize * *width as usize + b.position.x as usize;
                let terrain = tiles[idx];
                assert!(
                    terrain.base_passable(),
                    "Scene {s}: building {:?} at ({},{}) on impassable {:?}",
                    b.kind,
                    b.position.x,
                    b.position.y,
                    terrain
                );
            }
        }
    }

    #[test]
    fn hero_units_on_passable_tiles() {
        for s in 1..=3 {
            let m = validate_scene(s);
            let MissionMap::Inline { width, tiles, .. } = &m.map else {
                panic!("Expected Inline map");
            };
            for u in &m.player_setup.units {
                let idx = u.position.y as usize * *width as usize + u.position.x as usize;
                let terrain = tiles[idx];
                assert!(
                    terrain.base_passable(),
                    "Scene {s}: unit {:?} at ({},{}) on impassable {:?}",
                    u.kind,
                    u.position.x,
                    u.position.y,
                    terrain
                );
            }
        }
    }

    #[test]
    fn default_scenario_fallback() {
        // Invalid scenarios should fall back to scene 1
        let m0 = build_cutscene_mission(0);
        let m99 = build_cutscene_mission(99);
        assert_eq!(m0.id, "cutscene_1");
        assert_eq!(m99.id, "cutscene_1");
    }

    #[test]
    fn cutscene_camera_centered() {
        let cam = cutscene_camera();
        assert_eq!(cam.focus, GridPos::new(20, 15));
        assert!((cam.zoom - 0.8).abs() < f32::EPSILON);
    }
}
