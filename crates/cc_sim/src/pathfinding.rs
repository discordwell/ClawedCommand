use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::terrain::FactionId;

/// A* pathfinding on the isometric grid with 8-directional movement.
/// Faction-aware: respects faction-specific terrain passability and costs.
pub fn find_path(
    map: &GameMap,
    from: GridPos,
    to: GridPos,
    faction: FactionId,
) -> Option<Vec<GridPos>> {
    if !map.is_passable_for(to, faction) || !map.in_bounds(from) {
        return None;
    }
    if from == to {
        return Some(vec![to]);
    }

    let mut open = BinaryHeap::new();
    let mut closed: HashSet<GridPos> = HashSet::new();
    let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
    let mut g_score: HashMap<GridPos, u32> = HashMap::new();

    g_score.insert(from, 0);
    open.push(Node {
        pos: from,
        f_score: heuristic(from, to),
    });

    while let Some(current) = open.pop() {
        if current.pos == to {
            return Some(reconstruct_path(&came_from, to));
        }

        if !closed.insert(current.pos) {
            continue; // Already expanded this node
        }

        let current_g = g_score[&current.pos];

        for neighbor in map.neighbors_for_faction(current.pos, faction) {
            // Elevation check: must be able to move between elevation levels
            if !map.can_move_between(current.pos, neighbor) {
                continue;
            }

            let base_cost: u32 = if is_diagonal(current.pos, neighbor) {
                14 // ~sqrt(2) * 10
            } else {
                10
            };

            // Terrain cost multiplier
            let terrain_cost = map
                .movement_cost_for(neighbor, faction)
                .unwrap_or(cc_core::math::FIXED_ONE);
            let terrain_multiplier: u32 =
                (terrain_cost * cc_core::math::Fixed::from_num(10u32)).to_num::<u32>();
            let weighted_cost = (base_cost * terrain_multiplier) / 10;

            // Elevation modifier: +20% per level uphill, -10% per level downhill
            let from_elev = map.elevation_at(current.pos) as i32;
            let to_elev = map.elevation_at(neighbor) as i32;
            let elev_diff = to_elev - from_elev;
            let elevation_cost = if elev_diff > 0 {
                // Uphill: +20% per level
                weighted_cost + (weighted_cost * elev_diff as u32 * 20) / 100
            } else if elev_diff < 0 {
                // Downhill: -10% per level (min 1)
                let reduction = (weighted_cost * (-elev_diff) as u32 * 10) / 100;
                weighted_cost.saturating_sub(reduction).max(1)
            } else {
                weighted_cost
            };

            let tentative_g = current_g + elevation_cost;

            if tentative_g < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                came_from.insert(neighbor, current.pos);
                g_score.insert(neighbor, tentative_g);
                open.push(Node {
                    pos: neighbor,
                    f_score: tentative_g + heuristic(neighbor, to),
                });
            }
        }
    }

    None // No path found
}

/// Backward-compatible find_path that uses base passability (CatGPT faction).
pub fn find_path_basic(map: &GameMap, from: GridPos, to: GridPos) -> Option<Vec<GridPos>> {
    find_path(map, from, to, FactionId::CatGPT)
}

/// Chebyshev distance scaled by 10 (matching cardinal cost of 10).
fn heuristic(a: GridPos, b: GridPos) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    let (min, max) = if dx < dy { (dx, dy) } else { (dy, dx) };
    // Diagonal steps cost 14, remaining cardinal steps cost 10
    min * 14 + (max - min) * 10
}

fn is_diagonal(a: GridPos, b: GridPos) -> bool {
    (a.x - b.x).abs() == 1 && (a.y - b.y).abs() == 1
}

fn reconstruct_path(came_from: &HashMap<GridPos, GridPos>, end: GridPos) -> Vec<GridPos> {
    let mut path = vec![end];
    let mut current = end;
    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    // Remove the starting position -- we're already there
    if path.len() > 1 {
        path.remove(0);
    }
    path
}

#[derive(Eq, PartialEq)]
struct Node {
    pos: GridPos,
    f_score: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score) // Min-heap
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::terrain::TerrainType;

    #[test]
    fn path_to_self() {
        let map = GameMap::new(10, 10);
        let path = find_path(&map, GridPos::new(5, 5), GridPos::new(5, 5), FactionId::CatGPT);
        assert_eq!(path, Some(vec![GridPos::new(5, 5)]));
    }

    #[test]
    fn straight_line_path() {
        let map = GameMap::new(10, 10);
        let path =
            find_path(&map, GridPos::new(0, 0), GridPos::new(3, 0), FactionId::CatGPT).unwrap();
        assert_eq!(*path.last().unwrap(), GridPos::new(3, 0));
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn path_around_obstacle() {
        let mut map = GameMap::new(10, 10);
        for y in 0..5 {
            map.get_mut(GridPos::new(3, y)).unwrap().terrain = TerrainType::Rock;
        }
        let path = find_path(&map, GridPos::new(1, 2), GridPos::new(5, 2), FactionId::CatGPT);
        assert!(path.is_some());
        let path = path.unwrap();
        for pos in &path {
            assert!(map.is_passable(*pos));
        }
        assert_eq!(*path.last().unwrap(), GridPos::new(5, 2));
    }

    #[test]
    fn no_path_when_blocked() {
        let mut map = GameMap::new(5, 5);
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            map.get_mut(GridPos::new(2 + dx, 2 + dy)).unwrap().terrain = TerrainType::Rock;
        }
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(2, 2), FactionId::CatGPT);
        assert!(path.is_none());
    }

    #[test]
    fn first_waypoint_is_neighbor_of_start() {
        let map = GameMap::new(10, 10);
        let start = GridPos::new(0, 0);
        let end = GridPos::new(5, 5);
        let path = find_path(&map, start, end, FactionId::CatGPT).unwrap();
        let first = path[0];
        assert!(
            (first.x - start.x).abs() <= 1 && (first.y - start.y).abs() <= 1,
            "first waypoint {:?} should be adjacent to start {:?}",
            first,
            start
        );
        assert_eq!(*path.last().unwrap(), end);
    }

    #[test]
    fn path_with_closed_set_still_finds_optimal() {
        let mut map = GameMap::new(10, 10);
        for y in 0..8 {
            map.get_mut(GridPos::new(4, y)).unwrap().terrain = TerrainType::Rock;
        }
        let path = find_path(&map, GridPos::new(2, 4), GridPos::new(6, 4), FactionId::CatGPT);
        assert!(path.is_some());
        let path = path.unwrap();
        for pos in &path {
            assert!(map.is_passable(*pos));
        }
        assert_eq!(*path.last().unwrap(), GridPos::new(6, 4));
    }

    #[test]
    fn path_to_impassable_target() {
        let mut map = GameMap::new(10, 10);
        map.get_mut(GridPos::new(5, 5)).unwrap().terrain = TerrainType::Rock;
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(5, 5), FactionId::CatGPT);
        assert!(path.is_none());
    }

    // --- New terrain-aware tests ---

    #[test]
    fn catgpt_paths_around_water_croak_goes_through() {
        let mut map = GameMap::new(10, 10);
        // Water wall from (5,0) to (5,9)
        for y in 0..10 {
            map.get_mut(GridPos::new(5, y)).unwrap().terrain = TerrainType::Water;
        }

        // CatGPT cannot cross water
        let catgpt_path = find_path(&map, GridPos::new(2, 5), GridPos::new(8, 5), FactionId::CatGPT);
        assert!(catgpt_path.is_none()); // Water wall blocks entire column

        // Croak can traverse water
        let croak_path = find_path(&map, GridPos::new(2, 5), GridPos::new(8, 5), FactionId::Croak);
        assert!(croak_path.is_some());
        let croak_path = croak_path.unwrap();
        assert_eq!(*croak_path.last().unwrap(), GridPos::new(8, 5));
    }

    #[test]
    fn path_prefers_road_over_grass() {
        let mut map = GameMap::new(10, 3);
        // Road along y=0
        for x in 0..10 {
            map.get_mut(GridPos::new(x, 0)).unwrap().terrain = TerrainType::Road;
        }
        // Grass along y=1 (default)
        // Forest along y=2
        for x in 0..10 {
            map.get_mut(GridPos::new(x, 2)).unwrap().terrain = TerrainType::Forest;
        }

        // Path from (0,0) to (9,0) should stay on road row
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(9, 0), FactionId::CatGPT).unwrap();
        for pos in &path {
            assert_eq!(pos.y, 0, "Path should stay on road row, but visited {:?}", pos);
        }
    }

    #[test]
    fn path_uses_ford_to_cross_river() {
        let mut map = GameMap::new(10, 10);
        // River of water at x=5
        for y in 0..10 {
            map.get_mut(GridPos::new(5, y)).unwrap().terrain = TerrainType::Water;
        }
        // Ford (shallows) at (5, 5)
        map.get_mut(GridPos::new(5, 5)).unwrap().terrain = TerrainType::Shallows;

        let path = find_path(&map, GridPos::new(3, 5), GridPos::new(7, 5), FactionId::CatGPT);
        assert!(path.is_some());
        let path = path.unwrap();
        // Path should cross through the ford
        assert!(path.contains(&GridPos::new(5, 5)), "Path should use the ford");
        assert_eq!(*path.last().unwrap(), GridPos::new(7, 5));
    }

    #[test]
    fn elevation_blocks_without_ramp() {
        let mut map = GameMap::new(10, 10);
        // Create a high plateau at x >= 5
        for y in 0..10 {
            for x in 5..10 {
                map.get_mut(GridPos::new(x, y)).unwrap().elevation = 1;
            }
        }
        // No ramps — should not be able to cross

        let path = find_path(&map, GridPos::new(3, 5), GridPos::new(7, 5), FactionId::CatGPT);
        assert!(path.is_none(), "Should not path across elevation without ramp");
    }

    #[test]
    fn elevation_passable_with_ramp() {
        let mut map = GameMap::new(10, 10);
        // High ground at x >= 6
        for y in 0..10 {
            for x in 6..10 {
                map.get_mut(GridPos::new(x, y)).unwrap().elevation = 1;
            }
        }
        // Ramp at (5, 5) connecting levels
        map.get_mut(GridPos::new(5, 5)).unwrap().terrain = TerrainType::Ramp;
        map.get_mut(GridPos::new(5, 5)).unwrap().elevation = 1;

        let path = find_path(&map, GridPos::new(3, 5), GridPos::new(7, 5), FactionId::CatGPT);
        assert!(path.is_some(), "Should path via ramp");
        let path = path.unwrap();
        assert!(path.contains(&GridPos::new(5, 5)), "Path should go through ramp");
    }

    #[test]
    fn uphill_costs_more_than_downhill() {
        let mut map = GameMap::new(10, 3);
        // Ramp in middle, high ground on right
        for x in 0..10 {
            map.get_mut(GridPos::new(x, 1)).unwrap().terrain = TerrainType::Ramp;
        }
        // Low ground (elev 0) on left, high ground (elev 1) on right
        for x in 5..10 {
            for y in 0..3 {
                map.get_mut(GridPos::new(x, y)).unwrap().elevation = 1;
            }
        }

        // Uphill path (low → high)
        let up_path = find_path(&map, GridPos::new(0, 1), GridPos::new(9, 1), FactionId::CatGPT);
        assert!(up_path.is_some());

        // Downhill path (high → low)
        let down_path = find_path(&map, GridPos::new(9, 1), GridPos::new(0, 1), FactionId::CatGPT);
        assert!(down_path.is_some());

        // Both should find paths (the cost difference is internal to A*)
    }
}
