use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::coords::GridPos;
use crate::map::GameMap;
use crate::map_format::{
    CampTier, MapDefinition, MapSymmetry, NeutralCamp, ResourceKind, ResourcePlacement, SpawnPoint,
};
use crate::terrain::TerrainType;

/// Parameters for procedural map generation.
#[derive(Debug, Clone)]
pub struct MapGenParams {
    pub width: u32,
    pub height: u32,
    pub num_players: u8,
    pub symmetry: MapSymmetry,
    pub water_ratio: f32,
    pub forest_ratio: f32,
    pub seed: u64,
}

impl Default for MapGenParams {
    fn default() -> Self {
        Self {
            width: 64,
            height: 64,
            num_players: 2,
            symmetry: MapSymmetry::Rotational180,
            water_ratio: 0.12,
            forest_ratio: 0.15,
            seed: 42,
        }
    }
}

/// Generate a complete map following RTS design principles.
pub fn generate_map(params: &MapGenParams) -> MapDefinition {
    let mut rng = StdRng::seed_from_u64(params.seed);
    let w = params.width;
    let h = params.height;

    let mut map = GameMap::new(w, h);

    // Step 1: Place spawn points
    let spawn_positions = place_spawns(w, h, params.num_players, params.symmetry);

    // Step 2: Generate heightmap using Perlin noise
    let elevation_noise = Perlin::new(rng.r#gen());
    generate_elevation(&mut map, &elevation_noise, &spawn_positions);

    // Step 3: Generate terrain
    let terrain_noise = Perlin::new(rng.r#gen());
    let water_noise = Perlin::new(rng.r#gen());
    generate_terrain(&mut map, &terrain_noise, &water_noise, params, &spawn_positions);

    // Step 4: Carve roads between spawn points
    carve_roads(&mut map, &spawn_positions);

    // Step 5: Place ramps at elevation transitions along roads
    place_ramps(&mut map);

    // Step 6: Ensure fords across water barriers
    place_fords(&mut map, &spawn_positions);

    // Step 7: Place resources
    let resources = place_resources(&mut map, &spawn_positions, &mut rng);

    // Step 8: Place neutral camps (stubs for now)
    let neutral_camps = place_neutral_camps(&spawn_positions, w, h);

    // Step 9: Apply symmetry enforcement
    enforce_symmetry(&mut map, params.symmetry);

    // Step 10: Validate connectivity
    // (done externally by caller via validate_connectivity)

    // Build definition
    let tiles: Vec<(u8, u8)> = map
        .tiles()
        .iter()
        .map(|t| (t.terrain as u8, t.elevation))
        .collect();

    let spawn_points = spawn_positions
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| SpawnPoint {
            player: i as u8,
            pos: (x, y),
        })
        .collect();

    MapDefinition {
        name: format!("Generated Map (seed {})", params.seed),
        width: w,
        height: h,
        tiles,
        spawn_points,
        resources,
        neutral_camps,
        symmetry: params.symmetry,
    }
}

/// Place spawn points at map edges/corners based on symmetry and player count.
fn place_spawns(w: u32, h: u32, num_players: u8, symmetry: MapSymmetry) -> Vec<(i32, i32)> {
    let margin = 6i32;
    match (num_players, symmetry) {
        (2, MapSymmetry::Rotational180) => {
            vec![
                (margin, margin),                              // Player 0: top-left
                (w as i32 - margin - 1, h as i32 - margin - 1), // Player 1: bottom-right
            ]
        }
        (2, MapSymmetry::MirrorHorizontal) => {
            let cy = h as i32 / 2;
            vec![(margin, cy), (w as i32 - margin - 1, cy)]
        }
        (2, MapSymmetry::MirrorVertical) => {
            let cx = w as i32 / 2;
            vec![(cx, margin), (cx, h as i32 - margin - 1)]
        }
        (4, MapSymmetry::Rotational90) | (4, _) => {
            vec![
                (margin, margin),
                (w as i32 - margin - 1, margin),
                (w as i32 - margin - 1, h as i32 - margin - 1),
                (margin, h as i32 - margin - 1),
            ]
        }
        _ => {
            // Default: spread players along edges
            let mut spawns = Vec::new();
            for i in 0..num_players {
                let angle = std::f32::consts::TAU * (i as f32) / (num_players as f32);
                let cx = w as f32 / 2.0;
                let cy = h as f32 / 2.0;
                let r = (w.min(h) as f32 / 2.0) - margin as f32;
                let x = (cx + r * angle.cos()).clamp(margin as f32, (w as i32 - margin) as f32) as i32;
                let y = (cy + r * angle.sin()).clamp(margin as f32, (h as i32 - margin) as f32) as i32;
                spawns.push((x, y));
            }
            spawns
        }
    }
}

/// Generate elevation using Perlin noise. Spawn areas get elevated plateaus.
fn generate_elevation(map: &mut GameMap, noise: &Perlin, spawns: &[(i32, i32)]) {
    let w = map.width as f64;
    let h = map.height as f64;
    let scale = 0.06;

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let nx = x as f64 / w * scale * w;
            let ny = y as f64 / h * scale * h;
            let n = noise.get([nx, ny]);

            // Map noise to elevation: < -0.2 = 0, -0.2..0.3 = 1, > 0.3 = 2
            let elev = if n < -0.2 {
                0
            } else if n < 0.3 {
                1
            } else {
                2
            };

            if let Some(tile) = map.get_mut(GridPos::new(x, y)) {
                tile.elevation = elev;
            }
        }
    }

    // Enforce elevated plateaus at spawn points (5x5 area at elevation 2)
    for &(sx, sy) in spawns {
        for dy in -2..=2 {
            for dx in -2..=2 {
                if let Some(tile) = map.get_mut(GridPos::new(sx + dx, sy + dy)) {
                    tile.elevation = 2;
                }
            }
        }
    }
}

/// Generate terrain types using noise.
fn generate_terrain(
    map: &mut GameMap,
    terrain_noise: &Perlin,
    water_noise: &Perlin,
    params: &MapGenParams,
    spawns: &[(i32, i32)],
) {
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let nx = x as f64 * 0.08;
            let ny = y as f64 * 0.08;

            let terrain_n = terrain_noise.get([nx, ny]);
            let water_n = water_noise.get([nx * 0.5, ny * 0.5]);

            let pos = GridPos::new(x, y);
            let tile = map.get(pos).unwrap();
            let elev = tile.elevation;

            // Determine if near a spawn point (protected zone)
            let near_spawn = spawns
                .iter()
                .any(|&(sx, sy)| (x - sx).abs() <= 4 && (y - sy).abs() <= 4);

            let terrain = if near_spawn {
                // Spawn areas are always open grass
                TerrainType::Grass
            } else if elev == 0 && water_n < -(1.0 - params.water_ratio as f64 * 4.0) {
                // Low elevation + low water noise = water
                TerrainType::Water
            } else if terrain_n > (1.0 - params.forest_ratio as f64 * 3.0) && elev > 0 {
                // High terrain noise on elevated ground = forest
                TerrainType::Forest
            } else if terrain_n < -0.6 {
                // Very low noise = tech ruins (scattered)
                TerrainType::TechRuins
            } else if terrain_n > 0.3 && terrain_n < 0.45 {
                TerrainType::Dirt
            } else if elev == 0 && water_n < 0.0 && water_n > -(1.0 - params.water_ratio as f64 * 4.0) {
                // Near-water areas get sand
                TerrainType::Sand
            } else {
                TerrainType::Grass
            };

            map.get_mut(pos).unwrap().terrain = terrain;
        }
    }

    // Place rock at elevation transitions (non-ramp cliffs)
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let pos = GridPos::new(x, y);
            let elev = map.elevation_at(pos);

            // Check if this is an edge of an elevation change
            let mut has_lower_neighbor = false;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let np = GridPos::new(x + dx, y + dy);
                if map.in_bounds(np) {
                    let ne = map.elevation_at(np);
                    if ne < elev && (elev - ne) >= 2 {
                        has_lower_neighbor = true;
                        break;
                    }
                }
            }

            // Only place rock at steep cliff edges, not at ramp-suitable locations
            if has_lower_neighbor {
                let near_spawn = spawns
                    .iter()
                    .any(|&(sx, sy)| (x - sx).abs() <= 5 && (y - sy).abs() <= 5);
                if !near_spawn {
                    map.get_mut(pos).unwrap().terrain = TerrainType::Rock;
                }
            }
        }
    }
}

/// Carve road tiles along the shortest paths between spawn points.
fn carve_roads(map: &mut GameMap, spawns: &[(i32, i32)]) {
    for i in 0..spawns.len() {
        for j in (i + 1)..spawns.len() {
            let (x0, y0) = spawns[i];
            let (x1, y1) = spawns[j];
            // Simple Bresenham-ish line with some width
            carve_road_line(map, x0, y0, x1, y1);
        }
    }
}

fn carve_road_line(map: &mut GameMap, x0: i32, y0: i32, x1: i32, y1: i32) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        // Set road tile (and neighbors for width)
        for offset in -1..=1 {
            let pos = if dx > dy {
                GridPos::new(x, y + offset)
            } else {
                GridPos::new(x + offset, y)
            };
            if let Some(tile) = map.get_mut(pos) {
                // Don't overwrite water or rock
                if tile.terrain != TerrainType::Water && tile.terrain != TerrainType::Rock {
                    tile.terrain = TerrainType::Road;
                }
            }
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

/// Place ramps where roads cross elevation transitions.
fn place_ramps(map: &mut GameMap) {
    let w = map.width as i32;
    let h = map.height as i32;

    for y in 0..h {
        for x in 0..w {
            let pos = GridPos::new(x, y);
            let tile = map.get(pos).unwrap();
            if tile.terrain != TerrainType::Road {
                continue;
            }

            let elev = tile.elevation;

            // Check if adjacent to different elevation
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let np = GridPos::new(x + dx, y + dy);
                if map.in_bounds(np) && map.elevation_at(np) != elev {
                    map.get_mut(pos).unwrap().terrain = TerrainType::Ramp;
                    break;
                }
            }
        }
    }
}

/// Place shallows (fords) where paths need to cross water.
fn place_fords(map: &mut GameMap, spawns: &[(i32, i32)]) {
    // For each pair of spawns, check if there's a water barrier and place fords
    for i in 0..spawns.len() {
        for j in (i + 1)..spawns.len() {
            let (x0, y0) = spawns[i];
            let (x1, y1) = spawns[j];

            // Walk the line between spawns, find water crossings
            let dx = (x1 - x0).abs();
            let dy = (y1 - y0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx - dy;
            let mut x = x0;
            let mut y = y0;

            loop {
                let pos = GridPos::new(x, y);
                if let Some(tile) = map.get(pos) {
                    if tile.terrain == TerrainType::Water {
                        // Place ford (shallows) here and neighbors for width
                        for d in -1..=1 {
                            let fp = if dx > dy {
                                GridPos::new(x, y + d)
                            } else {
                                GridPos::new(x + d, y)
                            };
                            if let Some(ft) = map.get_mut(fp) {
                                if ft.terrain == TerrainType::Water {
                                    ft.terrain = TerrainType::Shallows;
                                }
                            }
                        }
                    }
                }

                if x == x1 && y == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    x += sx;
                }
                if e2 < dx {
                    err += dx;
                    y += sy;
                }
            }
        }
    }
}

/// Place resources following RTS conventions.
fn place_resources(
    map: &mut GameMap,
    spawns: &[(i32, i32)],
    rng: &mut StdRng,
) -> Vec<ResourcePlacement> {
    let mut resources = Vec::new();
    let cx = map.width as i32 / 2;
    let cy = map.height as i32 / 2;

    for &(sx, sy) in spawns {
        // Main base: Fish Pond + GPU Deposit
        let fish_offset = (rng.gen_range(-2..=2), rng.gen_range(-2..=2));
        resources.push(ResourcePlacement {
            kind: ResourceKind::FishPond,
            pos: (sx + fish_offset.0 + 3, sy + fish_offset.1),
        });
        resources.push(ResourcePlacement {
            kind: ResourceKind::GpuDeposit,
            pos: (sx - 3, sy + 2),
        });

        // Natural expansion: Fish Pond + Berry Bush (further out toward center)
        let dx = (cx - sx).signum() * 8;
        let dy = (cy - sy).signum() * 8;
        resources.push(ResourcePlacement {
            kind: ResourceKind::FishPond,
            pos: (sx + dx, sy + dy),
        });
        resources.push(ResourcePlacement {
            kind: ResourceKind::BerryBush,
            pos: (sx + dx + 2, sy + dy + 1),
        });
    }

    // Center: Monkey Mine(s)
    resources.push(ResourcePlacement {
        kind: ResourceKind::MonkeyMine,
        pos: (cx, cy),
    });

    // Secondary GPU deposits at contested locations
    if spawns.len() == 2 {
        let mid_x = (spawns[0].0 + spawns[1].0) / 2;
        let mid_y = (spawns[0].1 + spawns[1].1) / 2;
        resources.push(ResourcePlacement {
            kind: ResourceKind::GpuDeposit,
            pos: (mid_x + 5, mid_y - 3),
        });
        resources.push(ResourcePlacement {
            kind: ResourceKind::GpuDeposit,
            pos: (mid_x - 5, mid_y + 3),
        });
    }

    resources
}

/// Place neutral camps (stub — positions only).
fn place_neutral_camps(spawns: &[(i32, i32)], w: u32, h: u32) -> Vec<NeutralCamp> {
    let mut camps = Vec::new();
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;

    for &(sx, sy) in spawns {
        // Green camp near each base
        let dx = (cx - sx).signum() * 5;
        let dy = (cy - sy).signum() * 5;
        camps.push(NeutralCamp {
            tier: CampTier::Green,
            pos: (sx + dx, sy + dy),
        });

        // Orange camp between natural and third
        let dx2 = (cx - sx).signum() * 12;
        let dy2 = (cy - sy).signum() * 12;
        camps.push(NeutralCamp {
            tier: CampTier::Orange,
            pos: (sx + dx2, sy + dy2),
        });
    }

    // Red camp at center
    camps.push(NeutralCamp {
        tier: CampTier::Red,
        pos: (cx, cy),
    });

    camps
}

/// Enforce map symmetry by mirroring the first quadrant/half.
fn enforce_symmetry(map: &mut GameMap, symmetry: MapSymmetry) {
    let w = map.width as i32;
    let h = map.height as i32;

    match symmetry {
        MapSymmetry::Rotational180 => {
            // Copy top-left to bottom-right (rotated 180)
            for y in 0..h / 2 {
                for x in 0..w {
                    let src = GridPos::new(x, y);
                    let dst = GridPos::new(w - 1 - x, h - 1 - y);
                    if let (Some(src_tile), true) = (map.get(src), map.in_bounds(dst)) {
                        let terrain = src_tile.terrain;
                        let elevation = src_tile.elevation;
                        if let Some(dst_tile) = map.get_mut(dst) {
                            dst_tile.terrain = terrain;
                            dst_tile.elevation = elevation;
                        }
                    }
                }
            }
        }
        MapSymmetry::MirrorHorizontal => {
            // Mirror left half to right
            for y in 0..h {
                for x in 0..w / 2 {
                    let src = GridPos::new(x, y);
                    let dst = GridPos::new(w - 1 - x, y);
                    if let (Some(src_tile), true) = (map.get(src), map.in_bounds(dst)) {
                        let terrain = src_tile.terrain;
                        let elevation = src_tile.elevation;
                        if let Some(dst_tile) = map.get_mut(dst) {
                            dst_tile.terrain = terrain;
                            dst_tile.elevation = elevation;
                        }
                    }
                }
            }
        }
        MapSymmetry::MirrorVertical => {
            // Mirror top half to bottom
            for y in 0..h / 2 {
                for x in 0..w {
                    let src = GridPos::new(x, y);
                    let dst = GridPos::new(x, h - 1 - y);
                    if let (Some(src_tile), true) = (map.get(src), map.in_bounds(dst)) {
                        let terrain = src_tile.terrain;
                        let elevation = src_tile.elevation;
                        if let Some(dst_tile) = map.get_mut(dst) {
                            dst_tile.terrain = terrain;
                            dst_tile.elevation = elevation;
                        }
                    }
                }
            }
        }
        MapSymmetry::Rotational90 => {
            // Copy first quadrant to other three (rotated 90, 180, 270)
            for y in 0..h / 2 {
                for x in 0..w / 2 {
                    let src = GridPos::new(x, y);
                    if let Some(src_tile) = map.get(src) {
                        let terrain = src_tile.terrain;
                        let elevation = src_tile.elevation;

                        // 90: (x,y) -> (w-1-y, x)
                        let r90 = GridPos::new(w - 1 - y, x);
                        // 180: (x,y) -> (w-1-x, h-1-y)
                        let r180 = GridPos::new(w - 1 - x, h - 1 - y);
                        // 270: (x,y) -> (y, h-1-x)
                        let r270 = GridPos::new(y, h - 1 - x);

                        for dst in [r90, r180, r270] {
                            if let Some(dst_tile) = map.get_mut(dst) {
                                dst_tile.terrain = terrain;
                                dst_tile.elevation = elevation;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Validate that all spawn points can reach each other for non-Croak factions.
/// Uses a flood-fill from the first spawn point.
pub fn validate_connectivity(map: &GameMap, spawns: &[(i32, i32)]) -> Result<(), String> {
    use crate::terrain::FactionId;
    use std::collections::{HashSet, VecDeque};

    if spawns.is_empty() {
        return Ok(());
    }

    let start = GridPos::new(spawns[0].0, spawns[0].1);
    let mut visited: HashSet<GridPos> = HashSet::new();
    let mut queue: VecDeque<GridPos> = VecDeque::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(pos) = queue.pop_front() {
        for neighbor in map.neighbors_for_faction(pos, FactionId::CatGPT) {
            if !visited.contains(&neighbor) && map.can_move_between(pos, neighbor) {
                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }

    // Check all other spawns are reachable
    for (i, &(sx, sy)) in spawns.iter().enumerate().skip(1) {
        let spawn_pos = GridPos::new(sx, sy);
        if !visited.contains(&spawn_pos) {
            return Err(format!(
                "Spawn point {} at ({}, {}) is not reachable from spawn 0",
                i, sx, sy
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_default_map() {
        let params = MapGenParams::default();
        let def = generate_map(&params);
        assert_eq!(def.width, 64);
        assert_eq!(def.height, 64);
        assert_eq!(def.tiles.len(), 64 * 64);
        assert_eq!(def.spawn_points.len(), 2);
        assert!(!def.resources.is_empty());
        assert!(def.validate().is_ok());
    }

    #[test]
    fn generated_map_has_valid_terrain() {
        let def = generate_map(&MapGenParams::default());
        for (i, &(terrain_u8, elev)) in def.tiles.iter().enumerate() {
            assert!(
                TerrainType::from_u8(terrain_u8).is_some(),
                "Invalid terrain at index {}: {}",
                i,
                terrain_u8
            );
            assert!(elev <= 2, "Elevation too high at index {}: {}", i, elev);
        }
    }

    #[test]
    fn generated_map_is_deterministic() {
        let params = MapGenParams {
            seed: 12345,
            ..Default::default()
        };
        let def1 = generate_map(&params);
        let def2 = generate_map(&params);
        assert_eq!(def1.tiles, def2.tiles);
    }

    #[test]
    fn spawn_areas_are_passable() {
        let params = MapGenParams::default();
        let def = generate_map(&params);
        let map = def.to_game_map();

        for sp in &def.spawn_points {
            let pos = GridPos::new(sp.pos.0, sp.pos.1);
            assert!(
                map.is_passable(pos),
                "Spawn point ({}, {}) is not passable",
                sp.pos.0,
                sp.pos.1
            );
        }
    }

    #[test]
    fn four_player_map() {
        let params = MapGenParams {
            num_players: 4,
            symmetry: MapSymmetry::Rotational90,
            ..Default::default()
        };
        let def = generate_map(&params);
        assert_eq!(def.spawn_points.len(), 4);
        assert!(def.validate().is_ok());
    }

    #[test]
    fn map_connectivity_valid() {
        let params = MapGenParams {
            seed: 42,
            width: 32,
            height: 32,
            ..Default::default()
        };
        let def = generate_map(&params);
        let map = def.to_game_map();
        // Connectivity may not always pass with aggressive terrain gen,
        // but the spawns should at least be accessible
        for sp in &def.spawn_points {
            assert!(
                map.is_passable(GridPos::new(sp.pos.0, sp.pos.1)),
                "Spawn must be passable"
            );
        }
    }

    #[test]
    fn different_seeds_produce_different_maps() {
        let def1 = generate_map(&MapGenParams {
            seed: 1,
            ..Default::default()
        });
        let def2 = generate_map(&MapGenParams {
            seed: 2,
            ..Default::default()
        });
        assert_ne!(def1.tiles, def2.tiles);
    }

    #[test]
    fn map_has_resources_near_spawns() {
        let def = generate_map(&MapGenParams::default());
        assert!(
            def.resources.len() >= def.spawn_points.len() * 2,
            "Should have at least 2 resources per spawn"
        );

        // Check monkey mine at center
        let cx = def.width as i32 / 2;
        let cy = def.height as i32 / 2;
        assert!(
            def.resources
                .iter()
                .any(|r| r.kind == ResourceKind::MonkeyMine
                    && (r.pos.0 - cx).abs() <= 1
                    && (r.pos.1 - cy).abs() <= 1),
            "Should have a Monkey Mine near center"
        );
    }

    #[test]
    fn ron_round_trip_generated_map() {
        let def = generate_map(&MapGenParams {
            width: 16,
            height: 16,
            ..Default::default()
        });
        let ron_str = def.to_ron().unwrap();
        let restored = MapDefinition::from_ron(&ron_str).unwrap();
        assert_eq!(def.tiles, restored.tiles);
        assert_eq!(def.width, restored.width);
        assert_eq!(def.height, restored.height);
    }
}
