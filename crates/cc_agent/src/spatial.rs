use std::collections::HashMap;

use cc_core::coords::GridPos;

use crate::snapshot::UnitSnapshot;

/// Grid-bucketed spatial hash for efficient area queries over unit snapshots.
/// Maps grid cells to indices into a snapshot vector.
pub struct SpatialIndex {
    cells: HashMap<GridPos, Vec<usize>>,
}

impl SpatialIndex {
    /// Build a spatial index from a slice of unit snapshots.
    /// Each unit is indexed by its GridPos.
    pub fn build(units: &[UnitSnapshot]) -> Self {
        let mut cells: HashMap<GridPos, Vec<usize>> = HashMap::new();
        for (idx, unit) in units.iter().enumerate() {
            cells.entry(unit.pos).or_default().push(idx);
        }
        SpatialIndex { cells }
    }

    /// Return indices of all units within a Chebyshev (square) radius of center.
    pub fn units_in_radius(&self, center: GridPos, radius: i32) -> Vec<usize> {
        let mut result = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let cell = GridPos::new(center.x + dx, center.y + dy);
                if let Some(indices) = self.cells.get(&cell) {
                    result.extend_from_slice(indices);
                }
            }
        }
        result
    }

    /// Return indices of units exactly at the given position.
    pub fn units_at(&self, pos: GridPos) -> &[usize] {
        self.cells.get(&pos).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Find the index of the nearest unit to center within max_radius.
    /// Uses expanding ring search for efficiency.
    pub fn nearest(
        &self,
        center: GridPos,
        max_radius: i32,
        units: &[UnitSnapshot],
    ) -> Option<usize> {
        for r in 0..=max_radius {
            let mut best_idx = None;
            let mut best_dist_sq = i64::MAX;

            // Only check the ring at distance r (not the interior, already checked)
            for dy in -r..=r {
                for dx in -r..=r {
                    // Skip interior cells (already searched)
                    if dx.abs() < r && dy.abs() < r {
                        continue;
                    }
                    let cell = GridPos::new(center.x + dx, center.y + dy);
                    if let Some(indices) = self.cells.get(&cell) {
                        for &idx in indices {
                            let u = &units[idx];
                            let ddx = (u.pos.x - center.x) as i64;
                            let ddy = (u.pos.y - center.y) as i64;
                            let dist_sq = ddx * ddx + ddy * ddy;
                            if dist_sq < best_dist_sq {
                                best_dist_sq = dist_sq;
                                best_idx = Some(idx);
                            }
                        }
                    }
                }
            }

            if best_idx.is_some() {
                return best_idx;
            }
        }
        None
    }

    /// Check if any unit exists within the given radius.
    pub fn any_in_radius(&self, center: GridPos, radius: i32) -> bool {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let cell = GridPos::new(center.x + dx, center.y + dy);
                if let Some(indices) = self.cells.get(&cell) {
                    if !indices.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::commands::EntityId;
    use cc_core::components::{AttackType, UnitKind};
    use cc_core::coords::WorldPos;
    use cc_core::math::fixed_from_i32;

    fn make_unit(id: u64, x: i32, y: i32) -> UnitSnapshot {
        UnitSnapshot {
            id: EntityId(id),
            kind: UnitKind::Hisser,
            pos: GridPos::new(x, y),
            world_pos: WorldPos::from_grid(GridPos::new(x, y)),
            owner: 0,
            health_current: fixed_from_i32(100),
            health_max: fixed_from_i32(100),
            speed: fixed_from_i32(1),
            attack_damage: fixed_from_i32(10),
            attack_range: fixed_from_i32(5),
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            is_moving: false,
            is_attacking: false,
            is_idle: true,
            is_dead: false,
        }
    }

    #[test]
    fn empty_index() {
        let units: Vec<UnitSnapshot> = vec![];
        let index = SpatialIndex::build(&units);
        assert!(index.units_at(GridPos::new(0, 0)).is_empty());
        assert!(index.units_in_radius(GridPos::new(0, 0), 5).is_empty());
        assert!(index.nearest(GridPos::new(0, 0), 10, &units).is_none());
    }

    #[test]
    fn units_at_exact_position() {
        let units = vec![
            make_unit(1, 5, 5),
            make_unit(2, 5, 5),
            make_unit(3, 10, 10),
        ];
        let index = SpatialIndex::build(&units);

        let at_5_5 = index.units_at(GridPos::new(5, 5));
        assert_eq!(at_5_5.len(), 2);
        assert!(at_5_5.contains(&0));
        assert!(at_5_5.contains(&1));

        let at_10_10 = index.units_at(GridPos::new(10, 10));
        assert_eq!(at_10_10.len(), 1);
        assert!(at_10_10.contains(&2));

        assert!(index.units_at(GridPos::new(0, 0)).is_empty());
    }

    #[test]
    fn units_in_radius_finds_nearby() {
        let units = vec![
            make_unit(1, 5, 5),
            make_unit(2, 7, 5),   // 2 tiles away
            make_unit(3, 20, 20), // far away
        ];
        let index = SpatialIndex::build(&units);

        let nearby = index.units_in_radius(GridPos::new(5, 5), 3);
        assert_eq!(nearby.len(), 2);
        assert!(nearby.contains(&0));
        assert!(nearby.contains(&1));
        assert!(!nearby.contains(&2));
    }

    #[test]
    fn units_in_radius_zero_only_exact() {
        let units = vec![
            make_unit(1, 5, 5),
            make_unit(2, 6, 5),
        ];
        let index = SpatialIndex::build(&units);

        let exact = index.units_in_radius(GridPos::new(5, 5), 0);
        assert_eq!(exact.len(), 1);
        assert!(exact.contains(&0));
    }

    #[test]
    fn nearest_finds_closest() {
        let units = vec![
            make_unit(1, 10, 10),
            make_unit(2, 5, 5),
            make_unit(3, 3, 3),
        ];
        let index = SpatialIndex::build(&units);

        let nearest = index.nearest(GridPos::new(4, 4), 20, &units);
        assert_eq!(nearest, Some(2)); // (3,3) is closest to (4,4)
    }

    #[test]
    fn nearest_returns_none_beyond_radius() {
        let units = vec![make_unit(1, 50, 50)];
        let index = SpatialIndex::build(&units);

        assert!(index.nearest(GridPos::new(0, 0), 5, &units).is_none());
    }

    #[test]
    fn any_in_radius_check() {
        let units = vec![make_unit(1, 5, 5)];
        let index = SpatialIndex::build(&units);

        assert!(index.any_in_radius(GridPos::new(5, 5), 0));
        assert!(index.any_in_radius(GridPos::new(4, 4), 2));
        assert!(!index.any_in_radius(GridPos::new(0, 0), 2));
    }

    #[test]
    fn negative_coordinates() {
        let units = vec![
            make_unit(1, -3, -5),
            make_unit(2, -2, -4),
        ];
        let index = SpatialIndex::build(&units);

        let nearby = index.units_in_radius(GridPos::new(-3, -5), 2);
        assert_eq!(nearby.len(), 2);
    }
}
