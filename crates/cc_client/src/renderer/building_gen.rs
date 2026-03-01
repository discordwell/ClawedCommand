use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::components::BuildingKind;

/// Resource holding building sprite image handles (art from disk or procedural fallback).
#[derive(Resource)]
pub struct BuildingSprites {
    /// One image handle per BuildingKind (indexed by building_kind_index).
    pub sprites: Vec<Handle<Image>>,
    /// True if at least one sprite was loaded from a PNG file on disk.
    pub art_loaded: bool,
}

/// All 48 building kinds in canonical order (8 per faction × 6 factions).
pub const ALL_BUILDING_KINDS: [BuildingKind; 48] = [
    // catGPT (Cats)
    BuildingKind::TheBox,
    BuildingKind::CatTree,
    BuildingKind::FishMarket,
    BuildingKind::LitterBox,
    BuildingKind::ServerRack,
    BuildingKind::ScratchingPost,
    BuildingKind::CatFlap,
    BuildingKind::LaserPointer,
    // The Murder (Corvids)
    BuildingKind::TheParliament,
    BuildingKind::Rookery,
    BuildingKind::CarrionCache,
    BuildingKind::AntennaArray,
    BuildingKind::Panopticon,
    BuildingKind::NestBox,
    BuildingKind::ThornHedge,
    BuildingKind::Watchtower,
    // The Clawed (Mice)
    BuildingKind::TheBurrow,
    BuildingKind::NestingBox,
    BuildingKind::SeedVault,
    BuildingKind::JunkTransmitter,
    BuildingKind::GnawLab,
    BuildingKind::WarrenExpansion,
    BuildingKind::Mousehole,
    BuildingKind::SqueakTower,
    // Seekers of the Deep (Badgers)
    BuildingKind::TheSett,
    BuildingKind::WarHollow,
    BuildingKind::BurrowDepot,
    BuildingKind::CoreTap,
    BuildingKind::ClawMarks,
    BuildingKind::DeepWarren,
    BuildingKind::BulwarkGate,
    BuildingKind::SlagThrower,
    // Croak (Axolotls)
    BuildingKind::TheGrotto,
    BuildingKind::SpawningPools,
    BuildingKind::LilyMarket,
    BuildingKind::SunkenServer,
    BuildingKind::FossilStones,
    BuildingKind::ReedBed,
    BuildingKind::TidalGate,
    BuildingKind::SporeTower,
    // LLAMA (Raccoons)
    BuildingKind::TheDumpster,
    BuildingKind::ScrapHeap,
    BuildingKind::ChopShop,
    BuildingKind::JunkServer,
    BuildingKind::TinkerBench,
    BuildingKind::TrashPile,
    BuildingKind::DumpsterRelay,
    BuildingKind::TetanusTower,
];

/// Map BuildingKind to array index (0..47).
/// Derives position from `ALL_BUILDING_KINDS` to keep a single source of truth.
pub fn building_kind_index(kind: BuildingKind) -> usize {
    ALL_BUILDING_KINDS
        .iter()
        .position(|&k| k == kind)
        .expect("BuildingKind not in ALL_BUILDING_KINDS")
}

/// Snake_case slug for file paths.
pub fn building_slug(kind: BuildingKind) -> &'static str {
    match kind {
        // catGPT
        BuildingKind::TheBox => "the_box",
        BuildingKind::CatTree => "cat_tree",
        BuildingKind::FishMarket => "fish_market",
        BuildingKind::LitterBox => "litter_box",
        BuildingKind::ServerRack => "server_rack",
        BuildingKind::ScratchingPost => "scratching_post",
        BuildingKind::CatFlap => "cat_flap",
        BuildingKind::LaserPointer => "laser_pointer",
        // Murder
        BuildingKind::TheParliament => "the_parliament",
        BuildingKind::Rookery => "rookery",
        BuildingKind::CarrionCache => "carrion_cache",
        BuildingKind::AntennaArray => "antenna_array",
        BuildingKind::Panopticon => "panopticon",
        BuildingKind::NestBox => "nest_box",
        BuildingKind::ThornHedge => "thorn_hedge",
        BuildingKind::Watchtower => "watchtower",
        // Clawed
        BuildingKind::TheBurrow => "the_burrow",
        BuildingKind::NestingBox => "nesting_box",
        BuildingKind::SeedVault => "seed_vault",
        BuildingKind::JunkTransmitter => "junk_transmitter",
        BuildingKind::GnawLab => "gnaw_lab",
        BuildingKind::WarrenExpansion => "warren_expansion",
        BuildingKind::Mousehole => "mousehole",
        BuildingKind::SqueakTower => "squeak_tower",
        // Seekers
        BuildingKind::TheSett => "the_sett",
        BuildingKind::WarHollow => "war_hollow",
        BuildingKind::BurrowDepot => "burrow_depot",
        BuildingKind::CoreTap => "core_tap",
        BuildingKind::ClawMarks => "claw_marks",
        BuildingKind::DeepWarren => "deep_warren",
        BuildingKind::BulwarkGate => "bulwark_gate",
        BuildingKind::SlagThrower => "slag_thrower",
        // Croak
        BuildingKind::TheGrotto => "the_grotto",
        BuildingKind::SpawningPools => "spawning_pools",
        BuildingKind::LilyMarket => "lily_market",
        BuildingKind::SunkenServer => "sunken_server",
        BuildingKind::FossilStones => "fossil_stones",
        BuildingKind::ReedBed => "reed_bed",
        BuildingKind::TidalGate => "tidal_gate",
        BuildingKind::SporeTower => "spore_tower",
        // LLAMA
        BuildingKind::TheDumpster => "the_dumpster",
        BuildingKind::ScrapHeap => "scrap_heap",
        BuildingKind::ChopShop => "chop_shop",
        BuildingKind::JunkServer => "junk_server",
        BuildingKind::TinkerBench => "tinker_bench",
        BuildingKind::TrashPile => "trash_pile",
        BuildingKind::DumpsterRelay => "dumpster_relay",
        BuildingKind::TetanusTower => "tetanus_tower",
    }
}

/// Asset path for a building sprite PNG (relative to `assets/`).
pub fn building_sprite_path(kind: BuildingKind) -> String {
    format!("sprites/buildings/{}.png", building_slug(kind))
}

/// Architectural role of a building (determines procedural shape and scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildingRole {
    Hq,
    Barracks,
    ResourceDepot,
    SupplyDepot,
    TechBuilding,
    Research,
    Garrison,
    DefenseTower,
}

/// Map building to its architectural role.
pub fn building_role(kind: BuildingKind) -> BuildingRole {
    match kind {
        // HQs
        BuildingKind::TheBox
        | BuildingKind::TheParliament
        | BuildingKind::TheBurrow
        | BuildingKind::TheSett
        | BuildingKind::TheGrotto
        | BuildingKind::TheDumpster => BuildingRole::Hq,
        // Barracks
        BuildingKind::CatTree
        | BuildingKind::Rookery
        | BuildingKind::NestingBox
        | BuildingKind::WarHollow
        | BuildingKind::SpawningPools
        | BuildingKind::ChopShop => BuildingRole::Barracks,
        // Resource Depots
        BuildingKind::FishMarket
        | BuildingKind::CarrionCache
        | BuildingKind::SeedVault
        | BuildingKind::BurrowDepot
        | BuildingKind::LilyMarket
        | BuildingKind::ScrapHeap => BuildingRole::ResourceDepot,
        // Supply Depots
        BuildingKind::LitterBox
        | BuildingKind::NestBox
        | BuildingKind::WarrenExpansion
        | BuildingKind::DeepWarren
        | BuildingKind::ReedBed
        | BuildingKind::TrashPile => BuildingRole::SupplyDepot,
        // Tech Buildings
        BuildingKind::ServerRack
        | BuildingKind::AntennaArray
        | BuildingKind::JunkTransmitter
        | BuildingKind::CoreTap
        | BuildingKind::SunkenServer
        | BuildingKind::JunkServer => BuildingRole::TechBuilding,
        // Research
        BuildingKind::ScratchingPost
        | BuildingKind::Panopticon
        | BuildingKind::GnawLab
        | BuildingKind::ClawMarks
        | BuildingKind::FossilStones
        | BuildingKind::TinkerBench => BuildingRole::Research,
        // Garrison / Gate / Wall
        BuildingKind::CatFlap
        | BuildingKind::ThornHedge
        | BuildingKind::Mousehole
        | BuildingKind::BulwarkGate
        | BuildingKind::TidalGate
        | BuildingKind::DumpsterRelay => BuildingRole::Garrison,
        // Defense Towers
        BuildingKind::LaserPointer
        | BuildingKind::Watchtower
        | BuildingKind::SqueakTower
        | BuildingKind::SlagThrower
        | BuildingKind::SporeTower
        | BuildingKind::TetanusTower => BuildingRole::DefenseTower,
    }
}

/// Display scale by building role.
/// When `art_loaded` is true, uses smaller scales for high-res art sprites.
/// When false, uses larger scales for small procedural sprites.
pub fn building_scale(kind: BuildingKind, art_loaded: bool) -> f32 {
    let role = building_role(kind);
    if art_loaded {
        match role {
            BuildingRole::Hq => 0.30,
            BuildingRole::Barracks => 0.25,
            BuildingRole::ResourceDepot => 0.22,
            BuildingRole::TechBuilding => 0.24,
            BuildingRole::Research => 0.22,
            BuildingRole::SupplyDepot => 0.20,
            BuildingRole::Garrison => 0.20,
            BuildingRole::DefenseTower => 0.18,
        }
    } else {
        match role {
            BuildingRole::Hq => 1.0,
            BuildingRole::Barracks => 0.9,
            BuildingRole::ResourceDepot => 0.85,
            BuildingRole::TechBuilding => 0.9,
            BuildingRole::Research => 0.85,
            BuildingRole::SupplyDepot => 0.8,
            BuildingRole::Garrison => 0.8,
            BuildingRole::DefenseTower => 0.75,
        }
    }
}

/// Base drawing size for procedural building sprites.
fn draw_size(role: BuildingRole) -> (usize, usize) {
    match role {
        BuildingRole::Hq => (28, 28),
        BuildingRole::Barracks => (24, 26),
        BuildingRole::ResourceDepot => (24, 22),
        BuildingRole::TechBuilding => (24, 26),
        BuildingRole::Research => (22, 24),
        BuildingRole::SupplyDepot => (22, 20),
        BuildingRole::Garrison => (22, 22),
        BuildingRole::DefenseTower => (18, 26),
    }
}

/// Final sprite dimensions (2× draw size for crisp close-up zoom).
fn sprite_size(role: BuildingRole) -> (usize, usize) {
    let (w, h) = draw_size(role);
    (w * 2, h * 2)
}

/// Generate building sprite handles at startup. Tries to load PNGs from disk first,
/// falls back to procedural generation for any missing sprites.
pub fn generate_building_sprites(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(48);
    let mut any_art_loaded = false;

    for kind in ALL_BUILDING_KINDS {
        let asset_path = building_sprite_path(kind);

        let use_disk = super::asset_exists_on_disk(&asset_path);

        if use_disk {
            handles.push(asset_server.load(asset_path));
            any_art_loaded = true;
        } else {
            let img = generate_building_image(kind);
            handles.push(images.add(img));
        }
    }

    commands.insert_resource(BuildingSprites {
        sprites: handles,
        art_loaded: any_art_loaded,
    });
}

/// Generate a procedural building sprite. Drawn at 1× resolution in neutral gray,
/// then upscaled 2× with nearest-neighbor. Team color applied via Sprite::color at runtime.
fn generate_building_image(kind: BuildingKind) -> Image {
    let role = building_role(kind);
    let (dw, dh) = draw_size(role);
    let (fw, fh) = sprite_size(role);
    let mut draw_data = vec![0u8; dw * dh * 4];

    draw_building_shape(&mut draw_data, dw, dh, kind, role);

    // Upscale 2× with nearest-neighbor
    let mut data = vec![0u8; fw * fh * 4];
    for fy in 0..fh {
        for fx in 0..fw {
            let sx = fx / 2;
            let sy = fy / 2;
            let src = (sy * dw + sx) * 4;
            let dst = (fy * fw + fx) * 4;
            data[dst..dst + 4].copy_from_slice(&draw_data[src..src + 4]);
        }
    }

    Image::new(
        Extent3d {
            width: fw as u32,
            height: fh as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

// ---------------------------------------------------------------------------
// Pixel drawing helpers
// ---------------------------------------------------------------------------

fn set_pixel(data: &mut [u8], w: usize, h: usize, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        let idx = (y as usize * w + x as usize) * 4;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
        data[idx + 3] = a;
    }
}

/// Fill a rectangle.
fn fill_rect(data: &mut [u8], w: usize, h: usize, x0: i32, y0: i32, rw: i32, rh: i32, r: u8, g: u8, b: u8) {
    for py in y0..y0 + rh {
        for px in x0..x0 + rw {
            set_pixel(data, w, h, px, py, r, g, b, 255);
        }
    }
}

/// Fill a circle.
fn fill_circle(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, radius: f32, r: u8, g: u8, b: u8) {
    let r2 = radius * radius;
    for py in 0..h {
        for px in 0..w {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                set_pixel(data, w, h, px as i32, py as i32, r, g, b, 255);
            }
        }
    }
}

/// Draw a building shape. Each role gets a distinct silhouette with a glyph.
fn draw_building_shape(data: &mut [u8], w: usize, h: usize, kind: BuildingKind, role: BuildingRole) {
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;

    match role {
        BuildingRole::Hq => {
            // Large rectangle with thick outline + star glyph
            fill_rect(data, w, h, 2, 3, w as i32 - 4, h as i32 - 5, 50, 50, 55); // outline
            fill_rect(data, w, h, 4, 5, w as i32 - 8, h as i32 - 9, 170, 170, 175); // body
            // Roof triangle
            for dy in 0..5 {
                let span = (w as i32 - 4) - dy * 2;
                if span > 0 {
                    fill_rect(data, w, h, 2 + dy, 3 - dy, span, 1, 140, 140, 150);
                }
            }
            // Star glyph
            set_pixel(data, w, h, cx, cy - 2, 255, 220, 50, 255);
            set_pixel(data, w, h, cx - 1, cy - 1, 255, 220, 50, 255);
            set_pixel(data, w, h, cx + 1, cy - 1, 255, 220, 50, 255);
            set_pixel(data, w, h, cx, cy, 255, 220, 50, 255);
            set_pixel(data, w, h, cx - 1, cy + 1, 255, 220, 50, 255);
            set_pixel(data, w, h, cx + 1, cy + 1, 255, 220, 50, 255);
        }
        BuildingRole::Barracks => {
            // Tall rectangle with battlements
            fill_rect(data, w, h, 3, 4, w as i32 - 6, h as i32 - 6, 50, 50, 55);
            fill_rect(data, w, h, 5, 6, w as i32 - 10, h as i32 - 10, 165, 165, 170);
            // Battlements on top
            for i in 0..4 {
                let bx = 4 + i * 5;
                fill_rect(data, w, h, bx, 2, 3, 3, 130, 130, 140);
            }
            // Sword glyph
            fill_rect(data, w, h, cx, cy - 3, 1, 7, 200, 200, 210);
            fill_rect(data, w, h, cx - 2, cy - 1, 5, 1, 200, 200, 210);
        }
        BuildingRole::ResourceDepot => {
            // Wide squat rectangle with crate-like lines
            fill_rect(data, w, h, 2, 5, w as i32 - 4, h as i32 - 7, 50, 50, 55);
            fill_rect(data, w, h, 4, 7, w as i32 - 8, h as i32 - 11, 160, 155, 145);
            // Crate cross lines
            fill_rect(data, w, h, cx, 7, 1, h as i32 - 11, 120, 115, 105);
            fill_rect(data, w, h, 4, cy, w as i32 - 8, 1, 120, 115, 105);
        }
        BuildingRole::SupplyDepot => {
            // Smaller rectangle with stacked boxes look
            fill_rect(data, w, h, 3, 4, w as i32 - 6, h as i32 - 6, 50, 50, 55);
            fill_rect(data, w, h, 5, 6, w as i32 - 10, h as i32 - 10, 155, 155, 150);
            // Horizontal divider
            fill_rect(data, w, h, 5, cy, w as i32 - 10, 1, 120, 120, 115);
            // Small square on top
            fill_rect(data, w, h, cx - 3, 2, 6, 4, 140, 140, 135);
        }
        BuildingRole::TechBuilding => {
            // Rectangle with antenna/dish on top
            fill_rect(data, w, h, 3, 6, w as i32 - 6, h as i32 - 8, 50, 50, 55);
            fill_rect(data, w, h, 5, 8, w as i32 - 10, h as i32 - 12, 160, 165, 175);
            // Antenna
            fill_rect(data, w, h, cx, 1, 1, 7, 100, 100, 110);
            // Dish (small V)
            set_pixel(data, w, h, cx - 2, 2, 180, 180, 190, 255);
            set_pixel(data, w, h, cx - 1, 3, 180, 180, 190, 255);
            set_pixel(data, w, h, cx + 1, 3, 180, 180, 190, 255);
            set_pixel(data, w, h, cx + 2, 2, 180, 180, 190, 255);
            // Blinking light
            set_pixel(data, w, h, cx, 1, 255, 80, 80, 255);
        }
        BuildingRole::Research => {
            // Rectangle with book/scroll glyph
            fill_rect(data, w, h, 3, 4, w as i32 - 6, h as i32 - 6, 50, 50, 55);
            fill_rect(data, w, h, 5, 6, w as i32 - 10, h as i32 - 10, 155, 160, 165);
            // Scroll/book shape
            fill_rect(data, w, h, cx - 3, cy - 3, 7, 7, 200, 195, 180);
            fill_rect(data, w, h, cx - 2, cy - 2, 5, 5, 240, 235, 220);
            // Page lines
            fill_rect(data, w, h, cx - 1, cy - 1, 3, 1, 120, 115, 100);
            fill_rect(data, w, h, cx - 1, cy + 1, 3, 1, 120, 115, 100);
        }
        BuildingRole::Garrison => {
            // Arch/gate shape
            fill_rect(data, w, h, 2, 3, w as i32 - 4, h as i32 - 4, 50, 50, 55);
            fill_rect(data, w, h, 4, 5, w as i32 - 8, h as i32 - 8, 155, 150, 145);
            // Gate arch (clear center-bottom to transparent)
            for py in (cy + 1)..(cy + 1 + h as i32 - cy - 3) {
                for px in (cx - 3)..(cx - 3 + 7) {
                    set_pixel(data, w, h, px, py, 0, 0, 0, 0);
                }
            }
            // Arch top (rounded via pixels)
            for dx in -3..=3i32 {
                let arch_y = cy + 1 - (3 - dx.unsigned_abs() as i32).max(0);
                set_pixel(data, w, h, cx + dx, arch_y, 50, 50, 55, 255);
            }
        }
        BuildingRole::DefenseTower => {
            // Narrow tall tower with pointed top
            fill_rect(data, w, h, 5, 6, w as i32 - 10, h as i32 - 8, 50, 50, 55);
            fill_rect(data, w, h, 6, 7, w as i32 - 12, h as i32 - 10, 170, 170, 175);
            // Pointed roof
            for dy in 0..6 {
                let half = 6 - dy;
                fill_rect(data, w, h, cx - half, 6 - dy, half * 2 + 1, 1, 140, 140, 150);
            }
            // Window slit
            fill_rect(data, w, h, cx, cy - 1, 1, 3, 60, 60, 80);
        }
    }

    // Add a tiny faction-specific accent mark based on the building kind
    draw_faction_accent(data, w, h, kind);
}

/// Draw a small faction-identifying accent mark in the bottom-right corner.
fn draw_faction_accent(data: &mut [u8], w: usize, h: usize, kind: BuildingKind) {
    let ax = w as i32 - 5;
    let ay = h as i32 - 5;

    match kind {
        // catGPT — small paw print (dot pattern)
        BuildingKind::TheBox | BuildingKind::CatTree | BuildingKind::FishMarket
        | BuildingKind::LitterBox | BuildingKind::ServerRack | BuildingKind::ScratchingPost
        | BuildingKind::CatFlap | BuildingKind::LaserPointer => {
            set_pixel(data, w, h, ax, ay + 1, 200, 180, 160, 255);
            set_pixel(data, w, h, ax - 1, ay, 200, 180, 160, 255);
            set_pixel(data, w, h, ax + 1, ay, 200, 180, 160, 255);
        }
        // Murder — small feather (diagonal line)
        BuildingKind::TheParliament | BuildingKind::Rookery | BuildingKind::CarrionCache
        | BuildingKind::AntennaArray | BuildingKind::Panopticon | BuildingKind::NestBox
        | BuildingKind::ThornHedge | BuildingKind::Watchtower => {
            for i in 0..3 {
                set_pixel(data, w, h, ax - 1 + i, ay - 1 + i, 80, 60, 100, 255);
            }
        }
        // Clawed — tiny whisker lines
        BuildingKind::TheBurrow | BuildingKind::NestingBox | BuildingKind::SeedVault
        | BuildingKind::JunkTransmitter | BuildingKind::GnawLab | BuildingKind::WarrenExpansion
        | BuildingKind::Mousehole | BuildingKind::SqueakTower => {
            set_pixel(data, w, h, ax - 1, ay, 160, 140, 120, 255);
            set_pixel(data, w, h, ax + 1, ay, 160, 140, 120, 255);
        }
        // Seekers — small claw mark (two slashes)
        BuildingKind::TheSett | BuildingKind::WarHollow | BuildingKind::BurrowDepot
        | BuildingKind::CoreTap | BuildingKind::ClawMarks | BuildingKind::DeepWarren
        | BuildingKind::BulwarkGate | BuildingKind::SlagThrower => {
            set_pixel(data, w, h, ax - 1, ay - 1, 140, 130, 110, 255);
            set_pixel(data, w, h, ax, ay, 140, 130, 110, 255);
            set_pixel(data, w, h, ax + 1, ay - 1, 140, 130, 110, 255);
        }
        // Croak — small water drop
        BuildingKind::TheGrotto | BuildingKind::SpawningPools | BuildingKind::LilyMarket
        | BuildingKind::SunkenServer | BuildingKind::FossilStones | BuildingKind::ReedBed
        | BuildingKind::TidalGate | BuildingKind::SporeTower => {
            set_pixel(data, w, h, ax, ay - 1, 100, 180, 200, 255);
            set_pixel(data, w, h, ax - 1, ay, 100, 180, 200, 255);
            set_pixel(data, w, h, ax, ay, 100, 180, 200, 255);
            set_pixel(data, w, h, ax + 1, ay, 100, 180, 200, 255);
        }
        // LLAMA — small gear/bolt (X shape)
        BuildingKind::TheDumpster | BuildingKind::ScrapHeap | BuildingKind::ChopShop
        | BuildingKind::JunkServer | BuildingKind::TinkerBench | BuildingKind::TrashPile
        | BuildingKind::DumpsterRelay | BuildingKind::TetanusTower => {
            set_pixel(data, w, h, ax - 1, ay - 1, 180, 160, 100, 255);
            set_pixel(data, w, h, ax + 1, ay - 1, 180, 160, 100, 255);
            set_pixel(data, w, h, ax, ay, 180, 160, 100, 255);
            set_pixel(data, w, h, ax - 1, ay + 1, 180, 160, 100, 255);
            set_pixel(data, w, h, ax + 1, ay + 1, 180, 160, 100, 255);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_building_kinds_has_48_entries() {
        assert_eq!(ALL_BUILDING_KINDS.len(), 48);
    }

    #[test]
    fn building_kind_index_covers_all() {
        for (i, kind) in ALL_BUILDING_KINDS.iter().enumerate() {
            assert_eq!(
                building_kind_index(*kind),
                i,
                "building_kind_index mismatch for {:?}",
                kind
            );
        }
    }

    #[test]
    fn building_sprite_paths_are_consistent() {
        for kind in ALL_BUILDING_KINDS {
            let path = building_sprite_path(kind);
            assert!(
                path.starts_with("sprites/buildings/"),
                "Path should be under sprites/buildings/: {path}"
            );
            assert!(path.ends_with(".png"), "Path should end with .png: {path}");
        }
    }

    #[test]
    fn building_slugs_are_unique() {
        let mut slugs: Vec<&str> = ALL_BUILDING_KINDS.iter().map(|k| building_slug(*k)).collect();
        let len_before = slugs.len();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), len_before, "Duplicate building slugs detected");
    }

    #[test]
    fn every_role_is_covered() {
        use std::collections::HashSet;
        let roles: HashSet<BuildingRole> = ALL_BUILDING_KINDS
            .iter()
            .map(|k| building_role(*k))
            .collect();
        assert!(roles.contains(&BuildingRole::Hq));
        assert!(roles.contains(&BuildingRole::Barracks));
        assert!(roles.contains(&BuildingRole::ResourceDepot));
        assert!(roles.contains(&BuildingRole::SupplyDepot));
        assert!(roles.contains(&BuildingRole::TechBuilding));
        assert!(roles.contains(&BuildingRole::Research));
        assert!(roles.contains(&BuildingRole::Garrison));
        assert!(roles.contains(&BuildingRole::DefenseTower));
    }

    #[test]
    fn procedural_images_are_nonzero() {
        for kind in ALL_BUILDING_KINDS {
            let img = generate_building_image(kind);
            let data = img.data.as_ref().expect("Image should have data");
            let has_nonzero = data.iter().any(|&b| b != 0);
            assert!(
                has_nonzero,
                "Procedural image for {:?} is all zeros",
                kind
            );
        }
    }

    #[test]
    fn hq_buildings_identified_correctly() {
        let hqs: Vec<BuildingKind> = ALL_BUILDING_KINDS
            .iter()
            .copied()
            .filter(|k| building_role(*k) == BuildingRole::Hq)
            .collect();
        assert_eq!(hqs.len(), 6, "Should have exactly 6 HQ buildings (one per faction)");
    }

    #[test]
    fn each_faction_has_8_buildings() {
        // catGPT = indices 0..8, Murder = 8..16, etc.
        for faction_start in (0..48).step_by(8) {
            let faction_kinds = &ALL_BUILDING_KINDS[faction_start..faction_start + 8];
            // First building in each faction should be HQ
            assert_eq!(
                building_role(faction_kinds[0]),
                BuildingRole::Hq,
                "First building of faction starting at index {} should be HQ",
                faction_start
            );
        }
    }
}
