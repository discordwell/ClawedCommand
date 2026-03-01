use cc_core::components::BuildingKind;
use cc_core::coords::GridPos;
use cc_core::mission::*;
use cc_core::terrain::TerrainType;

use crate::renderer::building_gen::ALL_BUILDING_KINDS;
use crate::setup;

/// Faction neighborhood centers on the 80×48 map (3 columns × 2 rows).
const FACTION_CENTERS: [(i32, i32); 6] = [
    (13, 13), // catGPT (cats)
    (40, 13), // Murder (corvids)
    (67, 13), // Clawed (mice)
    (13, 35), // Seekers (badgers)
    (40, 35), // Croak (axolotls)
    (67, 35), // LLAMA (raccoons)
];

/// Per-building offsets from the faction center (indexes 0..7 within each faction's 8-building slice).
/// Layout forms a 3×3 grid with the HQ at center (catGPT reference):
/// ```text
///   [Resource]  [Research]  [Barracks]
///   [Garrison]     [HQ]       [Tech]
///    [Supply]    [Tower]
/// ```
/// Note: Non-catGPT factions have different role orderings in ALL_BUILDING_KINDS,
/// so their buildings appear at different semantic positions. This is cosmetic only.
const BUILDING_OFFSETS: [(i32, i32); 8] = [
    (0, 0),   // 0: HQ (center)
    (4, -3),  // 1: upper right
    (-4, -3), // 2: upper left
    (-4, 3),  // 3: lower left
    (4, 0),   // 4: middle right
    (0, -3),  // 5: upper center
    (-4, 0),  // 6: middle left
    (0, 3),   // 7: lower center
];

const MAP_WIDTH: u32 = 80;
const MAP_HEIGHT: u32 = 48;

/// Build a showcase mission with all 48 buildings on a flat map.
pub fn build_showcase_mission() -> MissionDefinition {
    let total = (MAP_WIDTH * MAP_HEIGHT) as usize;
    let mut tiles = vec![TerrainType::Grass; total];
    let mut elevation = vec![1u8; total];

    // Add road avenues dividing the 3×2 grid
    // Horizontal road at y=24 (full width)
    for x in 0..MAP_WIDTH as i32 {
        set_tile(&mut tiles, &mut elevation, x, 24, TerrainType::Road, 1);
    }
    // Vertical roads at x=26-27 and x=52-53
    for y in 0..MAP_HEIGHT as i32 {
        set_tile(&mut tiles, &mut elevation, 26, y, TerrainType::Road, 1);
        set_tile(&mut tiles, &mut elevation, 27, y, TerrainType::Road, 1);
        set_tile(&mut tiles, &mut elevation, 52, y, TerrainType::Road, 1);
        set_tile(&mut tiles, &mut elevation, 53, y, TerrainType::Road, 1);
    }

    // Forest corners for visual interest
    for &(cx, cy) in &[(3, 3), (76, 3), (3, 44), (76, 44)] {
        stamp_diamond(&mut tiles, &mut elevation, cx, cy, 3, TerrainType::Forest, 1);
    }

    // Small decorative ponds between neighborhoods
    for &(px, py) in &[(26, 13), (52, 13), (26, 35), (52, 35)] {
        stamp_diamond(&mut tiles, &mut elevation, px, py, 1, TerrainType::Shallows, 0);
    }

    // Central intersection plaza (small water feature)
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            set_tile(&mut tiles, &mut elevation, 40 + dx, 24 + dy, TerrainType::Water, 0);
        }
    }

    // Build all 48 building spawns
    let mut buildings = Vec::with_capacity(48);
    for faction_idx in 0..6usize {
        let (cx, cy) = FACTION_CENTERS[faction_idx];
        let player_id = faction_idx as u8;
        let faction_start = faction_idx * 8;

        for local_idx in 0..8usize {
            let kind: BuildingKind = ALL_BUILDING_KINDS[faction_start + local_idx];
            let (dx, dy) = BUILDING_OFFSETS[local_idx];
            let gx = cx + dx;
            let gy = cy + dy;

            // Ensure the building tile is passable grass
            set_tile(&mut tiles, &mut elevation, gx, gy, TerrainType::Grass, 1);

            buildings.push(BuildingSpawn {
                kind,
                position: GridPos::new(gx, gy),
                player_id,
                pre_built: true,
            });
        }
    }

    MissionDefinition {
        id: "showcase".into(),
        name: "Building Showcase".into(),
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
            units: vec![],
            buildings,
            starting_food: 9999,
            starting_gpu: 9999,
            starting_nfts: 9999,
        },
        enemy_waves: vec![],
        objectives: vec![MissionObjective {
            id: "showcase".into(),
            description: "Admire the buildings".into(),
            primary: true,
            condition: ObjectiveCondition::Survive(999999),
        }],
        triggers: vec![],
        dialogue: vec![],
        briefing_text: "All 48 faction buildings on display.".into(),
        debrief_text: String::new(),
        ai_tool_tier: None,
        next_mission: NextMission::None,
        mutators: vec![],
    }
}

/// Stamp a diamond (manhattan distance) of tiles centered at (cx, cy).
fn stamp_diamond(
    tiles: &mut [TerrainType],
    elevation: &mut [u8],
    cx: i32,
    cy: i32,
    radius: i32,
    terrain: TerrainType,
    elev: u8,
) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx.abs() + dy.abs() <= radius {
                set_tile(tiles, elevation, cx + dx, cy + dy, terrain, elev);
            }
        }
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

    #[test]
    fn showcase_mission_validates() {
        let mission = build_showcase_mission();
        mission.validate().expect("Showcase mission should validate");
    }

    #[test]
    fn showcase_has_48_buildings() {
        let mission = build_showcase_mission();
        assert_eq!(mission.player_setup.buildings.len(), 48);
    }

    #[test]
    fn showcase_buildings_on_passable_tiles() {
        let mission = build_showcase_mission();
        let MissionMap::Inline { width, height, tiles, .. } = &mission.map else {
            panic!("Expected Inline map");
        };
        for bspawn in &mission.player_setup.buildings {
            let x = bspawn.position.x;
            let y = bspawn.position.y;
            assert!(x >= 0 && y >= 0);
            assert!((x as u32) < *width && (y as u32) < *height);
            let idx = y as usize * *width as usize + x as usize;
            assert_eq!(
                tiles[idx],
                TerrainType::Grass,
                "Building {:?} at ({},{}) should be on grass",
                bspawn.kind,
                x,
                y
            );
        }
    }

    #[test]
    fn showcase_6_factions_represented() {
        let mission = build_showcase_mission();
        let mut player_ids: Vec<u8> = mission
            .player_setup
            .buildings
            .iter()
            .map(|b| b.player_id)
            .collect();
        player_ids.sort();
        player_ids.dedup();
        assert_eq!(player_ids, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn showcase_no_building_overlap() {
        let mission = build_showcase_mission();
        let mut positions: Vec<(i32, i32)> = mission
            .player_setup
            .buildings
            .iter()
            .map(|b| (b.position.x, b.position.y))
            .collect();
        let count = positions.len();
        positions.sort();
        positions.dedup();
        assert_eq!(positions.len(), count, "Buildings should not overlap");
    }
}
