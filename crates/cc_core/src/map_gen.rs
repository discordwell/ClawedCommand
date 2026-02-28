use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::coords::GridPos;
use crate::map::GameMap;
use crate::map_format::{
    CampTier, MapDefinition, MapSymmetry, NeutralCamp, ResourceKind, ResourcePlacement, SpawnPoint,
};
use crate::terrain::TerrainType;

// ---------------------------------------------------------------------------
// Public enums
// ---------------------------------------------------------------------------

/// Map template defining macro-level strategic layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapTemplate {
    /// Central river valley with bridge chokepoints and elevated flanks.
    Valley,
    /// Open map with 3 roads crossing at a fortified center.
    Crossroads,
    /// Heavily fortified base areas with narrow approaches (stub → Valley).
    Fortress,
    /// Water-dominated map with island bases (stub → Valley).
    Islands,
}

impl Default for MapTemplate {
    fn default() -> Self {
        Self::Valley
    }
}

/// Predefined map dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapSize {
    Small,  // 48x48
    Medium, // 64x64
    Large,  // 96x96
}

impl Default for MapSize {
    fn default() -> Self {
        Self::Medium
    }
}

impl MapSize {
    /// Return (width, height) for this size.
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Small => (48, 48),
            Self::Medium => (64, 64),
            Self::Large => (96, 96),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneType {
    Base,
    Expansion,
    Contested,
    Center,
    Obstacle,
}

#[derive(Debug, Clone)]
struct TemplateZone {
    /// Normalized center (0.0..1.0).
    center: (f32, f32),
    /// Normalized radius.
    radius: f32,
    zone_type: ZoneType,
    terrain: TerrainType,
    elevation: u8,
}

#[derive(Debug, Clone)]
struct TemplateLane {
    /// Normalized waypoints (0.0..1.0).
    waypoints: Vec<(f32, f32)>,
    /// Normalized width.
    width: f32,
    terrain: TerrainType,
}

#[derive(Debug, Clone)]
struct TemplateLayout {
    zones: Vec<TemplateZone>,
    lanes: Vec<TemplateLane>,
    preferred_symmetry: MapSymmetry,
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

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
    pub template: MapTemplate,
    pub map_size: MapSize,
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
            template: MapTemplate::default(),
            map_size: MapSize::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Template definitions
// ---------------------------------------------------------------------------

fn valley_layout() -> TemplateLayout {
    TemplateLayout {
        zones: vec![
            // Base zones — opposite corners, elevated plateaus
            TemplateZone {
                center: (0.12, 0.12),
                radius: 0.10,
                zone_type: ZoneType::Base,
                terrain: TerrainType::Grass,
                elevation: 2,
            },
            TemplateZone {
                center: (0.88, 0.88),
                radius: 0.10,
                zone_type: ZoneType::Base,
                terrain: TerrainType::Grass,
                elevation: 2,
            },
            // Natural expansions — between base and river
            TemplateZone {
                center: (0.30, 0.30),
                radius: 0.08,
                zone_type: ZoneType::Expansion,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            TemplateZone {
                center: (0.70, 0.70),
                radius: 0.08,
                zone_type: ZoneType::Expansion,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            // Central river valley — water obstacle
            TemplateZone {
                center: (0.50, 0.50),
                radius: 0.14,
                zone_type: ZoneType::Obstacle,
                terrain: TerrainType::Water,
                elevation: 0,
            },
            // Contested zones — near river, flanking
            TemplateZone {
                center: (0.35, 0.60),
                radius: 0.07,
                zone_type: ZoneType::Contested,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            TemplateZone {
                center: (0.60, 0.35),
                radius: 0.07,
                zone_type: ZoneType::Contested,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            // Center control point
            TemplateZone {
                center: (0.50, 0.50),
                radius: 0.05,
                zone_type: ZoneType::Center,
                terrain: TerrainType::TechRuins,
                elevation: 0,
            },
        ],
        lanes: vec![
            // Main road: diagonal through center with bridge crossing
            TemplateLane {
                waypoints: vec![(0.12, 0.12), (0.35, 0.40), (0.50, 0.50), (0.65, 0.60), (0.88, 0.88)],
                width: 0.03,
                terrain: TerrainType::Road,
            },
            // Flanking dirt path along upper ridge
            TemplateLane {
                waypoints: vec![(0.12, 0.12), (0.25, 0.10), (0.50, 0.15), (0.75, 0.25), (0.88, 0.88)],
                width: 0.02,
                terrain: TerrainType::Dirt,
            },
            // Flanking dirt path along lower ridge
            TemplateLane {
                waypoints: vec![(0.12, 0.12), (0.10, 0.25), (0.15, 0.50), (0.25, 0.75), (0.88, 0.88)],
                width: 0.02,
                terrain: TerrainType::Dirt,
            },
        ],
        preferred_symmetry: MapSymmetry::Rotational180,
    }
}

fn crossroads_layout() -> TemplateLayout {
    TemplateLayout {
        zones: vec![
            // Base zones — left/right edges
            TemplateZone {
                center: (0.10, 0.50),
                radius: 0.10,
                zone_type: ZoneType::Base,
                terrain: TerrainType::Grass,
                elevation: 2,
            },
            TemplateZone {
                center: (0.90, 0.50),
                radius: 0.10,
                zone_type: ZoneType::Base,
                terrain: TerrainType::Grass,
                elevation: 2,
            },
            // Natural expansions
            TemplateZone {
                center: (0.25, 0.35),
                radius: 0.07,
                zone_type: ZoneType::Expansion,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            TemplateZone {
                center: (0.75, 0.65),
                radius: 0.07,
                zone_type: ZoneType::Expansion,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            // Center — TechRuins fortified position
            TemplateZone {
                center: (0.50, 0.50),
                radius: 0.08,
                zone_type: ZoneType::Center,
                terrain: TerrainType::TechRuins,
                elevation: 1,
            },
            // Forest obstacles between lanes
            TemplateZone {
                center: (0.35, 0.25),
                radius: 0.06,
                zone_type: ZoneType::Obstacle,
                terrain: TerrainType::Forest,
                elevation: 1,
            },
            TemplateZone {
                center: (0.65, 0.75),
                radius: 0.06,
                zone_type: ZoneType::Obstacle,
                terrain: TerrainType::Forest,
                elevation: 1,
            },
            // Rock obstacles
            TemplateZone {
                center: (0.35, 0.75),
                radius: 0.05,
                zone_type: ZoneType::Obstacle,
                terrain: TerrainType::Rock,
                elevation: 2,
            },
            TemplateZone {
                center: (0.65, 0.25),
                radius: 0.05,
                zone_type: ZoneType::Obstacle,
                terrain: TerrainType::Rock,
                elevation: 2,
            },
            // Contested zones
            TemplateZone {
                center: (0.35, 0.50),
                radius: 0.06,
                zone_type: ZoneType::Contested,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
            TemplateZone {
                center: (0.65, 0.50),
                radius: 0.06,
                zone_type: ZoneType::Contested,
                terrain: TerrainType::Grass,
                elevation: 1,
            },
        ],
        lanes: vec![
            // Main horizontal road
            TemplateLane {
                waypoints: vec![(0.10, 0.50), (0.35, 0.50), (0.50, 0.50), (0.65, 0.50), (0.90, 0.50)],
                width: 0.03,
                terrain: TerrainType::Road,
            },
            // Upper diagonal road
            TemplateLane {
                waypoints: vec![(0.10, 0.50), (0.25, 0.35), (0.50, 0.50)],
                width: 0.025,
                terrain: TerrainType::Road,
            },
            TemplateLane {
                waypoints: vec![(0.50, 0.50), (0.75, 0.35), (0.90, 0.50)],
                width: 0.025,
                terrain: TerrainType::Road,
            },
            // Lower diagonal road
            TemplateLane {
                waypoints: vec![(0.10, 0.50), (0.25, 0.65), (0.50, 0.50)],
                width: 0.025,
                terrain: TerrainType::Road,
            },
            TemplateLane {
                waypoints: vec![(0.50, 0.50), (0.75, 0.65), (0.90, 0.50)],
                width: 0.025,
                terrain: TerrainType::Road,
            },
        ],
        preferred_symmetry: MapSymmetry::MirrorHorizontal,
    }
}

fn get_layout(template: MapTemplate) -> TemplateLayout {
    match template {
        MapTemplate::Valley => valley_layout(),
        MapTemplate::Crossroads => crossroads_layout(),
        // Stubs delegate to Valley
        MapTemplate::Fortress => valley_layout(),
        MapTemplate::Islands => valley_layout(),
    }
}

// ---------------------------------------------------------------------------
// Main generation entry point
// ---------------------------------------------------------------------------

/// Generate a complete map following RTS design principles using template-based layouts.
pub fn generate_map(params: &MapGenParams) -> MapDefinition {
    let mut rng = StdRng::seed_from_u64(params.seed);

    // Step 1: Resolve size — prefer explicit width/height if overridden from defaults,
    // otherwise use map_size enum. This preserves backward compat with callers that set
    // width/height directly (e.g., the harness).
    let (w, h) = if params.width != 64 || params.height != 64 {
        (params.width, params.height)
    } else {
        params.map_size.dimensions()
    };

    // Step 2: Get template layout
    let layout = get_layout(params.template);

    let mut map = GameMap::new(w, h);

    // Step 3: Extract spawn positions from Base zones
    let spawns = place_spawns_from_layout(&layout, w, h, params.num_players, layout.preferred_symmetry);

    // Step 4: Paint zones — fill terrain + elevation from zone definitions
    paint_zones(&mut map, &layout, w, h);

    // Step 5: Carve lanes — multi-waypoint with circular brush
    carve_lanes(&mut map, &layout, w, h);

    // Step 6: Apply noise detail within non-structural zones
    let noise1 = Perlin::new(rng.r#gen());
    let noise2 = Perlin::new(rng.r#gen());
    apply_noise_detail(&mut map, &noise1, &noise2, params, &spawns);

    // Step 7: Enforce symmetry
    let symmetry = layout.preferred_symmetry;
    enforce_symmetry(&mut map, symmetry);

    // Step 8: Re-carve lanes post-symmetry to ensure they survive
    carve_lanes(&mut map, &layout, w, h);

    // Step 8b: Re-paint center features that symmetry enforcement may have bisected
    paint_center_features(&mut map, &layout, w, h);

    // Step 9: Place ramps at elevation transitions on lane paths
    place_ramps_on_lanes(&mut map, &layout, w, h);

    // Step 10: Place fords where lanes cross water
    place_fords_on_lanes(&mut map, &layout, w, h);

    // Step 11: Sculpt base areas — elevated grass plateaus with rock walls
    sculpt_base_areas(&mut map, &spawns);

    // Step 12: Place tiered resources
    let resources = place_tiered_resources(&map, &spawns, w, h);

    // Step 13: Place tiered camps
    let neutral_camps = place_tiered_camps(&spawns, w, h);

    // Step 14: Validate + repair connectivity
    repair_connectivity(&mut map, &spawns);

    // Build definition
    let tiles: Vec<(u8, u8)> = map
        .tiles()
        .iter()
        .map(|t| (t.terrain as u8, t.elevation))
        .collect();

    let spawn_points = spawns
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| SpawnPoint {
            player: i as u8,
            pos: (x, y),
        })
        .collect();

    MapDefinition {
        name: format!("Generated {:?} Map (seed {})", params.template, params.seed),
        width: w,
        height: h,
        tiles,
        spawn_points,
        resources,
        neutral_camps,
        symmetry,
        template: Some(params.template),
        map_size: Some(params.map_size),
    }
}

// ---------------------------------------------------------------------------
// Pipeline helpers
// ---------------------------------------------------------------------------

/// Extract spawn positions from Base zones in the template layout.
fn place_spawns_from_layout(
    layout: &TemplateLayout,
    w: u32,
    h: u32,
    num_players: u8,
    symmetry: MapSymmetry,
) -> Vec<(i32, i32)> {
    let base_zones: Vec<_> = layout.zones.iter().filter(|z| z.zone_type == ZoneType::Base).collect();

    if base_zones.len() >= num_players as usize {
        // Use base zone centers
        base_zones
            .iter()
            .take(num_players as usize)
            .map(|z| {
                let x = (z.center.0 * w as f32).round() as i32;
                let y = (z.center.1 * h as f32).round() as i32;
                // Margin 5 ensures 7x7 base plateau + 1-tile rock walls fit in-bounds
                (x.clamp(5, w as i32 - 6), y.clamp(5, h as i32 - 6))
            })
            .collect()
    } else {
        // Fallback: derive from symmetry
        let margin = 6i32;
        match (num_players, symmetry) {
            (2, MapSymmetry::Rotational180) => {
                vec![
                    (margin, margin),
                    (w as i32 - margin - 1, h as i32 - margin - 1),
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
            (4, _) => {
                vec![
                    (margin, margin),
                    (w as i32 - margin - 1, margin),
                    (w as i32 - margin - 1, h as i32 - margin - 1),
                    (margin, h as i32 - margin - 1),
                ]
            }
            _ => {
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
}

/// Paint terrain and elevation from template zone definitions.
fn paint_zones(map: &mut GameMap, layout: &TemplateLayout, w: u32, h: u32) {
    // Fill base terrain: grass at elevation 1
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if let Some(tile) = map.get_mut(GridPos::new(x, y)) {
                tile.terrain = TerrainType::Grass;
                tile.elevation = 1;
            }
        }
    }

    // Paint zones from template (later zones override earlier ones)
    for zone in &layout.zones {
        let cx = (zone.center.0 * w as f32) as i32;
        let cy = (zone.center.1 * h as f32) as i32;
        let r = (zone.radius * w.max(h) as f32) as i32;

        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= r * r {
                    let pos = GridPos::new(cx + dx, cy + dy);
                    if let Some(tile) = map.get_mut(pos) {
                        tile.terrain = zone.terrain;
                        tile.elevation = zone.elevation;
                    }
                }
            }
        }
    }
}

/// Walk all sampled points along a lane's waypoint segments, calling `callback` at each (x, y).
fn walk_lane_points(lane: &TemplateLane, w: u32, h: u32, mut callback: impl FnMut(i32, i32)) {
    for seg in lane.waypoints.windows(2) {
        let (x0, y0) = (seg[0].0 * w as f32, seg[0].1 * h as f32);
        let (x1, y1) = (seg[1].0 * w as f32, seg[1].1 * h as f32);

        let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let steps = (dist * 2.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps.max(1) as f32;
            let px = (x0 + (x1 - x0) * t) as i32;
            let py = (y0 + (y1 - y0) * t) as i32;
            callback(px, py);
        }
    }
}

/// Carve lanes using multi-waypoint linear interpolation with circular brush.
fn carve_lanes(map: &mut GameMap, layout: &TemplateLayout, w: u32, h: u32) {
    for lane in &layout.lanes {
        let brush_r = (lane.width * w.max(h) as f32).max(1.0) as i32;
        let terrain = lane.terrain;

        walk_lane_points(lane, w, h, |px, py| {
            for dy in -brush_r..=brush_r {
                for dx in -brush_r..=brush_r {
                    if dx * dx + dy * dy <= brush_r * brush_r {
                        let pos = GridPos::new(px + dx, py + dy);
                        if let Some(tile) = map.get_mut(pos) {
                            tile.terrain = terrain;
                        }
                    }
                }
            }
        });
    }
}

/// Re-paint any template zone whose center is at the map midpoint.
/// Symmetry enforcement copies one half onto the other, which bisects features
/// straddling the centre line. Because centre features are inherently symmetric,
/// repainting them as complete circles restores the intended shape without
/// breaking balance.
fn paint_center_features(map: &mut GameMap, layout: &TemplateLayout, w: u32, h: u32) {
    let mid_x = 0.5f32;
    let mid_y = 0.5f32;
    let threshold = 0.02f32;

    for zone in &layout.zones {
        if (zone.center.0 - mid_x).abs() < threshold && (zone.center.1 - mid_y).abs() < threshold {
            let cx = (zone.center.0 * w as f32) as i32;
            let cy = (zone.center.1 * h as f32) as i32;
            let r = (zone.radius * w.max(h) as f32) as i32;

            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        let pos = GridPos::new(cx + dx, cy + dy);
                        if let Some(tile) = map.get_mut(pos) {
                            tile.terrain = zone.terrain;
                            tile.elevation = zone.elevation;
                        }
                    }
                }
            }
        }
    }
}

/// Apply Perlin noise detail within non-structural areas.
fn apply_noise_detail(
    map: &mut GameMap,
    terrain_noise: &Perlin,
    water_noise: &Perlin,
    params: &MapGenParams,
    spawns: &[(i32, i32)],
) {
    let w = map.width as i32;
    let h = map.height as i32;

    for y in 0..h {
        for x in 0..w {
            let pos = GridPos::new(x, y);
            let tile = map.get(pos).unwrap();

            // Skip structural tiles: roads, water, rock, ramps, tech ruins
            if matches!(
                tile.terrain,
                TerrainType::Road | TerrainType::Water | TerrainType::Rock
                    | TerrainType::Ramp | TerrainType::TechRuins | TerrainType::Shallows
            ) {
                continue;
            }

            // Skip near spawns
            let near_spawn = spawns
                .iter()
                .any(|&(sx, sy)| (x - sx).abs() <= 5 && (y - sy).abs() <= 5);
            if near_spawn {
                continue;
            }

            let nx = x as f64 * 0.08;
            let ny = y as f64 * 0.08;
            // Mirror coordinates for 180° rotational symmetry
            let mx = (w - 1 - x) as f64 * 0.08;
            let my = (h - 1 - y) as f64 * 0.08;
            // Average noise at (x,y) and its rotational counterpart
            let tn = (terrain_noise.get([nx, ny]) + terrain_noise.get([mx, my])) * 0.5;
            let wn = (water_noise.get([nx * 0.5, ny * 0.5]) + water_noise.get([mx * 0.5, my * 0.5])) * 0.5;

            let terrain = if tn > (1.0 - params.forest_ratio as f64 * 3.0) && tile.elevation > 0 {
                TerrainType::Forest
            } else if tn < -0.6 && tile.elevation > 0 {
                // Scattered tech ruins
                TerrainType::TechRuins
            } else if tile.elevation == 0 && wn < 0.0 {
                // Sand near water areas
                TerrainType::Sand
            } else if tn > 0.3 && tn < 0.45 {
                TerrainType::Dirt
            } else {
                // Keep existing painted terrain
                continue;
            };

            map.get_mut(pos).unwrap().terrain = terrain;
        }
    }
}

/// Sculpt base areas: 7x7 grass plateau at elevation 2 with rock walls on border-facing sides.
fn sculpt_base_areas(map: &mut GameMap, spawns: &[(i32, i32)]) {
    let w = map.width as i32;
    let h = map.height as i32;
    let cx = w / 2;
    let cy = h / 2;

    for &(sx, sy) in spawns {
        // 7x7 grass plateau
        for dy in -3..=3 {
            for dx in -3..=3 {
                let pos = GridPos::new(sx + dx, sy + dy);
                if let Some(tile) = map.get_mut(pos) {
                    tile.terrain = TerrainType::Grass;
                    tile.elevation = 2;
                }
            }
        }

        // Rock walls on border-facing sides (away from center)
        // Determine which edges face the map border (away from center)
        let wall_left = sx < cx;
        let wall_top = sy < cy;

        // Place rock on the border-facing edges with a gap entrance toward center
        for i in -3i32..=3 {
            // Left/right wall (x-facing border)
            let wall_dx = if wall_left { -4 } else { 4 };
            // Skip center tile for gap entrance
            if i.abs() > 1 {
                let pos = GridPos::new(sx + wall_dx, sy + i);
                if let Some(tile) = map.get_mut(pos) {
                    tile.terrain = TerrainType::Rock;
                    tile.elevation = 2;
                }
            }

            // Top/bottom wall (y-facing border)
            let wall_dy = if wall_top { -4 } else { 4 };
            if i.abs() > 1 {
                let pos = GridPos::new(sx + i, sy + wall_dy);
                if let Some(tile) = map.get_mut(pos) {
                    tile.terrain = TerrainType::Rock;
                    tile.elevation = 2;
                }
            }
        }
    }
}

/// Place ramps at elevation transitions along lane paths.
fn place_ramps_on_lanes(map: &mut GameMap, layout: &TemplateLayout, w: u32, h: u32) {
    for lane in &layout.lanes {
        walk_lane_points(lane, w, h, |px, py| {
            let pos = GridPos::new(px, py);
            let Some(tile) = map.get(pos) else { return };
            let elev = tile.elevation;

            let at_transition = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|&(dx, dy)| {
                    let np = GridPos::new(px + dx, py + dy);
                    map.in_bounds(np) && map.elevation_at(np) != elev
                });

            if at_transition {
                if let Some(tile) = map.get_mut(pos) {
                    tile.terrain = TerrainType::Ramp;
                }
            }
        });
    }
}

/// Place fords (shallows) where lanes cross water tiles.
fn place_fords_on_lanes(map: &mut GameMap, layout: &TemplateLayout, w: u32, h: u32) {
    for lane in &layout.lanes {
        let brush_r = (lane.width * w.max(h) as f32).max(1.0) as i32;

        walk_lane_points(lane, w, h, |px, py| {
            for dy in -brush_r..=brush_r {
                for dx in -brush_r..=brush_r {
                    if dx * dx + dy * dy <= brush_r * brush_r {
                        let pos = GridPos::new(px + dx, py + dy);
                        if let Some(tile) = map.get_mut(pos) {
                            if tile.terrain == TerrainType::Water {
                                tile.terrain = TerrainType::Shallows;
                            }
                        }
                    }
                }
            }
        });
    }
}

/// Place tiered resources following RTS conventions.
fn place_tiered_resources(
    map: &GameMap,
    spawns: &[(i32, i32)],
    w: u32,
    h: u32,
) -> Vec<ResourcePlacement> {
    let mut resources = Vec::new();
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;

    for &(sx, sy) in spawns {
        // T0 (safe): FishPond + GpuDeposit within 5 tiles of spawn
        let fish_pos = find_passable_near(map, sx + 3, sy + 1, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::FishPond,
            pos: fish_pos,
        });
        let gpu_pos = find_passable_near(map, sx - 3, sy + 2, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::GpuDeposit,
            pos: gpu_pos,
        });

        // T1 (natural): FishPond + BerryBush ~12 tiles toward center
        let dx_to_center = (cx - sx).signum();
        let dy_to_center = (cy - sy).signum();
        let t1_x = sx + dx_to_center * 12;
        let t1_y = sy + dy_to_center * 12;
        let t1_fish = find_passable_near(map, t1_x, t1_y, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::FishPond,
            pos: t1_fish,
        });
        let t1_berry = find_passable_near(map, t1_x + 2, t1_y + 1, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::BerryBush,
            pos: t1_berry,
        });

        // T2 (contested): GpuDeposit + BerryBush ~60% toward center
        let t2_x = sx + (cx - sx) * 6 / 10;
        let t2_y = sy + (cy - sy) * 6 / 10;
        let t2_gpu = find_passable_near(map, t2_x + 3, t2_y - 2, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::GpuDeposit,
            pos: t2_gpu,
        });
        let t2_berry = find_passable_near(map, t2_x - 2, t2_y + 3, 5);
        resources.push(ResourcePlacement {
            kind: ResourceKind::BerryBush,
            pos: t2_berry,
        });
    }

    // T3 (objective): MonkeyMine at map center
    let mm_pos = find_passable_near(map, cx, cy, 8);
    resources.push(ResourcePlacement {
        kind: ResourceKind::MonkeyMine,
        pos: mm_pos,
    });

    resources
}

/// Find a passable tile near (x, y), spiraling outward up to `radius`.
fn find_passable_near(map: &GameMap, x: i32, y: i32, radius: i32) -> (i32, i32) {
    // First check the exact position
    let pos = GridPos::new(x, y);
    if map.in_bounds(pos) && map.is_passable(pos) {
        return (x, y);
    }

    // Spiral outward
    for r in 1..=radius {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() == r || dy.abs() == r {
                    let pos = GridPos::new(x + dx, y + dy);
                    if map.in_bounds(pos) && map.is_passable(pos) {
                        return (x + dx, y + dy);
                    }
                }
            }
        }
    }

    // Fallback: return original position
    (x, y)
}

/// Place tiered neutral camps.
fn place_tiered_camps(spawns: &[(i32, i32)], w: u32, h: u32) -> Vec<NeutralCamp> {
    let mut camps = Vec::new();
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;

    for &(sx, sy) in spawns {
        // Green camp near base (~5 tiles toward center)
        let dx = (cx - sx).signum() * 5;
        let dy = (cy - sy).signum() * 5;
        camps.push(NeutralCamp {
            tier: CampTier::Green,
            pos: (sx + dx, sy + dy),
        });

        // Orange camp at contested (~60% toward center)
        let dx2 = (cx - sx) * 6 / 10;
        let dy2 = (cy - sy) * 6 / 10;
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

/// BFS flood fill from `start`, returning all reachable positions (CatGPT passability).
fn flood_fill_passable(map: &GameMap, start: GridPos) -> std::collections::HashSet<GridPos> {
    use crate::terrain::FactionId;
    use std::collections::{HashSet, VecDeque};

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

    visited
}

/// Validate connectivity and repair with emergency dirt paths if needed.
fn repair_connectivity(map: &mut GameMap, spawns: &[(i32, i32)]) {
    if spawns.len() < 2 {
        return;
    }

    let start = GridPos::new(spawns[0].0, spawns[0].1);
    let mut visited = flood_fill_passable(map, start);

    for &(sx, sy) in spawns.iter().skip(1) {
        let spawn_pos = GridPos::new(sx, sy);
        if visited.contains(&spawn_pos) {
            continue;
        }

        // Emergency path from this spawn to spawn 0
        let (tx, ty) = spawns[0];
        carve_emergency_path(map, sx, sy, tx, ty);

        // Re-flood to update visited set after carving
        visited = flood_fill_passable(map, start);
    }
}

/// Carve an emergency dirt path between two points using Bresenham with a 3-wide brush.
fn carve_emergency_path(map: &mut GameMap, x0: i32, y0: i32, x1: i32, y1: i32) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        // 3-wide brush
        for d in -1..=1 {
            let pos1 = GridPos::new(x + d, y);
            let pos2 = GridPos::new(x, y + d);
            for pos in [pos1, pos2] {
                if let Some(tile) = map.get_mut(pos) {
                    // base_passable() returns false for Rock and Water
                    if !tile.terrain.base_passable() {
                        tile.terrain = TerrainType::Dirt;
                    }
                }
            }
        }

        // Place ramp if elevation transition
        let pos = GridPos::new(x, y);
        if let Some(tile) = map.get(pos) {
            let elev = tile.elevation;
            let mut needs_ramp = false;
            for (ndx, ndy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let np = GridPos::new(x + ndx, y + ndy);
                if map.in_bounds(np) && map.elevation_at(np) != elev {
                    needs_ramp = true;
                    break;
                }
            }
            if needs_ramp {
                if let Some(tile) = map.get_mut(pos) {
                    tile.terrain = TerrainType::Ramp;
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

// ---------------------------------------------------------------------------
// Symmetry (kept from original)
// ---------------------------------------------------------------------------

/// Enforce map symmetry by mirroring the first quadrant/half.
fn enforce_symmetry(map: &mut GameMap, symmetry: MapSymmetry) {
    let w = map.width as i32;
    let h = map.height as i32;

    match symmetry {
        MapSymmetry::Rotational180 => {
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
            debug_assert_eq!(w, h, "Rotational90 symmetry requires square maps");
            for y in 0..h / 2 {
                for x in 0..w / 2 {
                    let src = GridPos::new(x, y);
                    if let Some(src_tile) = map.get(src) {
                        let terrain = src_tile.terrain;
                        let elevation = src_tile.elevation;

                        let r90 = GridPos::new(w - 1 - y, x);
                        let r180 = GridPos::new(w - 1 - x, h - 1 - y);
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

// ---------------------------------------------------------------------------
// Connectivity validation (public, kept from original)
// ---------------------------------------------------------------------------

/// Validate that all spawn points can reach each other for non-Croak factions.
pub fn validate_connectivity(map: &GameMap, spawns: &[(i32, i32)]) -> Result<(), String> {
    if spawns.is_empty() {
        return Ok(());
    }

    let start = GridPos::new(spawns[0].0, spawns[0].1);
    let visited = flood_fill_passable(map, start);

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_default_map() {
        let params = MapGenParams::default();
        let def = generate_map(&params);
        // Default is Valley/Medium = 64x64
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
        // 2 players × 6 resources each + 1 MonkeyMine = 13
        assert!(
            def.resources.len() >= 13,
            "Should have at least 13 resources for 2 players, got {}",
            def.resources.len()
        );

        // Check monkey mine near center
        let cx = def.width as i32 / 2;
        let cy = def.height as i32 / 2;
        assert!(
            def.resources
                .iter()
                .any(|r| r.kind == ResourceKind::MonkeyMine
                    && (r.pos.0 - cx).abs() <= 8
                    && (r.pos.1 - cy).abs() <= 8),
            "Should have a Monkey Mine near center"
        );
    }

    #[test]
    fn ron_round_trip_generated_map() {
        let def = generate_map(&MapGenParams {
            map_size: MapSize::Small,
            ..Default::default()
        });
        let ron_str = def.to_ron().unwrap();
        let restored = MapDefinition::from_ron(&ron_str).unwrap();
        assert_eq!(def.tiles, restored.tiles);
        assert_eq!(def.width, restored.width);
        assert_eq!(def.height, restored.height);
        assert_eq!(def.template, restored.template);
        assert_eq!(def.map_size, restored.map_size);
    }

    // --- New tests ---

    #[test]
    fn map_size_dimensions_correct() {
        assert_eq!(MapSize::Small.dimensions(), (48, 48));
        assert_eq!(MapSize::Medium.dimensions(), (64, 64));
        assert_eq!(MapSize::Large.dimensions(), (96, 96));
    }

    #[test]
    fn all_template_sizes_generate_valid_maps() {
        let templates = [MapTemplate::Valley, MapTemplate::Crossroads, MapTemplate::Fortress, MapTemplate::Islands];
        let sizes = [MapSize::Small, MapSize::Medium, MapSize::Large];

        for &template in &templates {
            for &map_size in &sizes {
                let (w, h) = map_size.dimensions();
                let params = MapGenParams {
                    template,
                    map_size,
                    seed: 42,
                    ..Default::default()
                };
                let def = generate_map(&params);
                assert_eq!(def.width, w, "Width mismatch for {:?}/{:?}", template, map_size);
                assert_eq!(def.height, h, "Height mismatch for {:?}/{:?}", template, map_size);
                assert!(
                    def.validate().is_ok(),
                    "Validation failed for {:?}/{:?}: {:?}",
                    template,
                    map_size,
                    def.validate()
                );

                // Every tile must be valid terrain with valid elevation
                for (i, &(terrain_u8, elev)) in def.tiles.iter().enumerate() {
                    assert!(
                        TerrainType::from_u8(terrain_u8).is_some(),
                        "Invalid terrain at index {} for {:?}/{:?}",
                        i, template, map_size
                    );
                    assert!(elev <= 2, "Elevation too high at index {} for {:?}/{:?}", i, template, map_size);
                }
            }
        }
    }

    #[test]
    fn valley_has_water_in_center() {
        let params = MapGenParams {
            template: MapTemplate::Valley,
            seed: 42,
            ..Default::default()
        };
        let def = generate_map(&params);
        let map = def.to_game_map();
        let cx = def.width as i32 / 2;
        let cy = def.height as i32 / 2;

        // Check a region around center for water/shallows presence
        let mut water_count = 0;
        let check_r = 8;
        for dy in -check_r..=check_r {
            for dx in -check_r..=check_r {
                let pos = GridPos::new(cx + dx, cy + dy);
                if let Some(t) = map.terrain_at(pos) {
                    if t.is_water() {
                        water_count += 1;
                    }
                }
            }
        }
        assert!(
            water_count > 10,
            "Valley should have water near center, found only {} water tiles",
            water_count
        );
    }

    #[test]
    fn crossroads_has_roads_through_center() {
        let params = MapGenParams {
            template: MapTemplate::Crossroads,
            symmetry: MapSymmetry::MirrorHorizontal,
            seed: 42,
            ..Default::default()
        };
        let def = generate_map(&params);
        let map = def.to_game_map();
        let cx = def.width as i32 / 2;
        let cy = def.height as i32 / 2;

        // Count road tiles near center
        let mut road_count = 0;
        let check_r = 5;
        for dy in -check_r..=check_r {
            for dx in -check_r..=check_r {
                let pos = GridPos::new(cx + dx, cy + dy);
                if let Some(t) = map.terrain_at(pos) {
                    if t == TerrainType::Road || t == TerrainType::TechRuins || t == TerrainType::Ramp {
                        road_count += 1;
                    }
                }
            }
        }
        assert!(
            road_count > 5,
            "Crossroads should have roads/ruins through center, found only {} road-like tiles",
            road_count
        );
    }

    #[test]
    fn tiered_resources_placed_correctly() {
        let params = MapGenParams::default();
        let def = generate_map(&params);

        // 2 players × 6 resources each + 1 MonkeyMine = 13
        assert!(
            def.resources.len() >= 13,
            "Expected at least 13 resources, got {}",
            def.resources.len()
        );

        // Must have at least 1 MonkeyMine
        assert!(
            def.resources.iter().any(|r| r.kind == ResourceKind::MonkeyMine),
            "Must have a MonkeyMine"
        );

        // Each spawn should have resources nearby (T0)
        for sp in &def.spawn_points {
            let nearby = def
                .resources
                .iter()
                .filter(|r| {
                    (r.pos.0 - sp.pos.0).abs() <= 8 && (r.pos.1 - sp.pos.1).abs() <= 8
                })
                .count();
            assert!(
                nearby >= 2,
                "Spawn ({}, {}) should have at least 2 nearby resources, got {}",
                sp.pos.0, sp.pos.1, nearby
            );
        }
    }

    #[test]
    fn base_areas_have_elevated_terrain() {
        let params = MapGenParams::default();
        let def = generate_map(&params);
        let map = def.to_game_map();

        for sp in &def.spawn_points {
            let elev = map.elevation_at(GridPos::new(sp.pos.0, sp.pos.1));
            assert_eq!(
                elev, 2,
                "Spawn ({}, {}) should be at elevation 2, got {}",
                sp.pos.0, sp.pos.1, elev
            );
        }
    }

    #[test]
    fn connectivity_all_templates() {
        let templates = [MapTemplate::Valley, MapTemplate::Crossroads, MapTemplate::Fortress, MapTemplate::Islands];

        for &template in &templates {
            let params = MapGenParams {
                template,
                seed: 42,
                ..Default::default()
            };
            let def = generate_map(&params);
            let map = def.to_game_map();
            let spawn_tuples: Vec<(i32, i32)> = def.spawn_points.iter().map(|sp| sp.pos).collect();

            assert!(
                validate_connectivity(&map, &spawn_tuples).is_ok(),
                "Connectivity failed for {:?}",
                template
            );
        }
    }

    #[test]
    fn four_player_crossroads() {
        let params = MapGenParams {
            template: MapTemplate::Crossroads,
            map_size: MapSize::Large,
            num_players: 4,
            symmetry: MapSymmetry::Rotational90,
            seed: 42,
            ..Default::default()
        };
        let def = generate_map(&params);
        assert_eq!(def.spawn_points.len(), 4);
        assert_eq!(def.width, 96);
        assert_eq!(def.height, 96);
        assert!(def.validate().is_ok());
    }

    #[test]
    fn valley_center_pool_is_not_bisected() {
        let params = MapGenParams {
            template: MapTemplate::Valley,
            seed: 42,
            ..Default::default()
        };
        let def = generate_map(&params);
        let map = def.to_game_map();
        let cy = def.height as i32 / 2;

        let mut above = 0u32;
        let mut below = 0u32;
        for y in 0..def.height as i32 {
            for x in 0..def.width as i32 {
                if let Some(t) = map.terrain_at(GridPos::new(x, y)) {
                    if t == TerrainType::Water {
                        if y < cy {
                            above += 1;
                        } else if y > cy {
                            below += 1;
                        }
                    }
                }
            }
        }

        // Both halves must have water, and counts should be within 20%
        assert!(above > 0, "No water tiles above center");
        assert!(below > 0, "No water tiles below center");
        let ratio = above.min(below) as f32 / above.max(below) as f32;
        assert!(
            ratio >= 0.80,
            "Water distribution is lopsided: above={above}, below={below}, ratio={ratio:.2}"
        );
    }
}
