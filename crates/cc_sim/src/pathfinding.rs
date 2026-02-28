use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use cc_core::coords::GridPos;
use cc_core::map::GameMap;

/// A* pathfinding on the isometric grid with 8-directional movement.
pub fn find_path(map: &GameMap, from: GridPos, to: GridPos) -> Option<Vec<GridPos>> {
    if !map.is_passable(to) || !map.in_bounds(from) {
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

        for neighbor in map.neighbors(current.pos) {
            let move_cost = if is_diagonal(current.pos, neighbor) {
                14 // ~sqrt(2) * 10
            } else {
                10
            };
            let tentative_g = current_g + move_cost;

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
    // Remove the starting position — we're already there
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

    #[test]
    fn path_to_self() {
        let map = GameMap::new(10, 10);
        let path = find_path(&map, GridPos::new(5, 5), GridPos::new(5, 5));
        assert_eq!(path, Some(vec![GridPos::new(5, 5)]));
    }

    #[test]
    fn straight_line_path() {
        let map = GameMap::new(10, 10);
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(3, 0)).unwrap();
        assert_eq!(*path.last().unwrap(), GridPos::new(3, 0));
        // Should be 3 steps: (1,0), (2,0), (3,0)
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn path_around_obstacle() {
        let mut map = GameMap::new(10, 10);
        // Wall from (3,0) to (3,4)
        for y in 0..5 {
            map.get_mut(GridPos::new(3, y)).unwrap().passable = false;
        }
        let path = find_path(&map, GridPos::new(1, 2), GridPos::new(5, 2));
        assert!(path.is_some());
        let path = path.unwrap();
        // Path should not cross the wall
        for pos in &path {
            assert!(map.is_passable(*pos));
        }
        assert_eq!(*path.last().unwrap(), GridPos::new(5, 2));
    }

    #[test]
    fn no_path_when_blocked() {
        let mut map = GameMap::new(5, 5);
        // Surround (2,2) completely
        for (dx, dy) in [
            (-1, -1), (0, -1), (1, -1),
            (-1, 0),           (1, 0),
            (-1, 1),  (0, 1),  (1, 1),
        ] {
            map.get_mut(GridPos::new(2 + dx, 2 + dy)).unwrap().passable = false;
        }
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(2, 2));
        assert!(path.is_none());
    }

    #[test]
    fn first_waypoint_is_neighbor_of_start() {
        let map = GameMap::new(10, 10);
        let start = GridPos::new(0, 0);
        let end = GridPos::new(5, 5);
        let path = find_path(&map, start, end).unwrap();
        // First waypoint should be adjacent to start (not the final destination)
        let first = path[0];
        assert!(
            (first.x - start.x).abs() <= 1 && (first.y - start.y).abs() <= 1,
            "first waypoint {:?} should be adjacent to start {:?}",
            first,
            start
        );
        // Last waypoint should be the destination
        assert_eq!(*path.last().unwrap(), end);
    }

    #[test]
    fn path_with_closed_set_still_finds_optimal() {
        // Ensure the closed set doesn't prevent finding paths
        let mut map = GameMap::new(10, 10);
        // Create a funnel that forces re-evaluation of paths
        for y in 0..8 {
            map.get_mut(GridPos::new(4, y)).unwrap().passable = false;
        }
        let path = find_path(&map, GridPos::new(2, 4), GridPos::new(6, 4));
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
        map.get_mut(GridPos::new(5, 5)).unwrap().passable = false;
        let path = find_path(&map, GridPos::new(0, 0), GridPos::new(5, 5));
        assert!(path.is_none());
    }
}
