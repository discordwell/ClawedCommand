//! Generate a map and output a PPM image for visual preview.
//! Usage: cargo run -p cc_core --bin map_preview -- [template] [seed] [size]
//! Templates: valley, crossroads, islands
//! Sizes: small, medium, large

use std::env;
use std::fs;
use std::io::Write;

use cc_core::map_gen::{MapGenParams, MapSize, MapTemplate, generate_map};
use cc_core::terrain::TerrainType;

fn terrain_color(terrain: TerrainType, elevation: u8) -> (u8, u8, u8) {
    let base = match terrain {
        TerrainType::Grass => (80, 160, 60),
        TerrainType::Dirt => (140, 110, 70),
        TerrainType::Sand => (210, 190, 130),
        TerrainType::Forest => (30, 100, 40),
        TerrainType::Water => (40, 80, 180),
        TerrainType::Shallows => (80, 140, 200),
        TerrainType::Rock => (100, 95, 90),
        TerrainType::Ramp => (150, 140, 100),
        TerrainType::Road => (160, 150, 130),
        TerrainType::TechRuins => (90, 70, 120),
        TerrainType::Concrete => (184, 179, 173),
        TerrainType::Linoleum => (199, 189, 166),
        TerrainType::CarpetTile => (115, 122, 140),
        TerrainType::MetalGrate => (97, 102, 107),
        TerrainType::DryWall => (217, 212, 204),
    };

    // Brighten by elevation
    let factor = 1.0 + elevation as f32 * 0.15;
    (
        (base.0 as f32 * factor).min(255.0) as u8,
        (base.1 as f32 * factor).min(255.0) as u8,
        (base.2 as f32 * factor).min(255.0) as u8,
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let template = match args.get(1).map(|s| s.as_str()) {
        Some("valley") => MapTemplate::Valley,
        Some("crossroads") => MapTemplate::Crossroads,
        Some("islands") => MapTemplate::Islands,
        Some("fortress") => MapTemplate::Fortress,
        _ => MapTemplate::Islands,
    };

    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(42);

    let map_size = match args.get(3).map(|s| s.as_str()) {
        Some("small") => MapSize::Small,
        Some("large") => MapSize::Large,
        _ => MapSize::Medium,
    };

    let (w, h) = map_size.dimensions();

    let params = MapGenParams {
        width: w,
        height: h,
        num_players: 2,
        symmetry: cc_core::map_format::MapSymmetry::Rotational180,
        water_ratio: 0.15,
        forest_ratio: 0.12,
        seed,
        template,
        map_size,
    };

    let map_def = generate_map(&params);

    // Scale factor for output image (each tile = scale x scale pixels)
    let scale = 8u32;
    let img_w = w * scale;
    let img_h = h * scale;

    let mut pixels = vec![0u8; (img_w * img_h * 3) as usize];

    for (i, &(terrain_u8, elevation)) in map_def.tiles.iter().enumerate() {
        let tx = i as u32 % w;
        let ty = i as u32 / w;
        let terrain = TerrainType::from_u8(terrain_u8).unwrap_or_default();
        let (r, g, b) = terrain_color(terrain, elevation);

        for sy in 0..scale {
            for sx in 0..scale {
                let px = tx * scale + sx;
                let py = ty * scale + sy;
                let idx = ((py * img_w + px) * 3) as usize;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
            }
        }
    }

    // Draw spawn points as red circles
    for sp in &map_def.spawn_points {
        let cx = sp.pos.0 as u32 * scale + scale / 2;
        let cy = sp.pos.1 as u32 * scale + scale / 2;
        let r = scale * 2;
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32)..=(r as i32) {
                if dx * dx + dy * dy <= (r as i32) * (r as i32) {
                    let px = (cx as i32 + dx) as u32;
                    let py = (cy as i32 + dy) as u32;
                    if px < img_w && py < img_h {
                        let idx = ((py * img_w + px) * 3) as usize;
                        pixels[idx] = 255;
                        pixels[idx + 1] = 50;
                        pixels[idx + 2] = 50;
                    }
                }
            }
        }
    }

    // Draw resources as colored diamonds
    for res in &map_def.resources {
        let cx = res.pos.0 as u32 * scale + scale / 2;
        let cy = res.pos.1 as u32 * scale + scale / 2;
        let (mr, mg, mb) = match res.kind {
            cc_core::map_format::ResourceKind::FishPond => (0, 200, 255),
            cc_core::map_format::ResourceKind::BerryBush => (200, 50, 200),
            cc_core::map_format::ResourceKind::GpuDeposit => (255, 200, 0),
            cc_core::map_format::ResourceKind::MonkeyMine => (255, 100, 0),
        };
        let s = (scale / 2 + 1) as i32;
        for dy in -s..=s {
            for dx in -s..=s {
                if dx.abs() + dy.abs() <= s {
                    let px = (cx as i32 + dx) as u32;
                    let py = (cy as i32 + dy) as u32;
                    if px < img_w && py < img_h {
                        let idx = ((py * img_w + px) * 3) as usize;
                        pixels[idx] = mr;
                        pixels[idx + 1] = mg;
                        pixels[idx + 2] = mb;
                    }
                }
            }
        }
    }

    // Draw neutral camps as white squares
    for camp in &map_def.neutral_camps {
        let cx = camp.pos.0 as u32 * scale + scale / 2;
        let cy = camp.pos.1 as u32 * scale + scale / 2;
        let s = scale / 2;
        for dy in -(s as i32)..=(s as i32) {
            for dx in -(s as i32)..=(s as i32) {
                let px = (cx as i32 + dx) as u32;
                let py = (cy as i32 + dy) as u32;
                if px < img_w && py < img_h {
                    let idx = ((py * img_w + px) * 3) as usize;
                    pixels[idx] = 255;
                    pixels[idx + 1] = 255;
                    pixels[idx + 2] = 255;
                }
            }
        }
    }

    // Write PPM
    let output_path = format!("map_preview_{:?}_{}.ppm", template, seed);
    let mut file = fs::File::create(&output_path).expect("Failed to create output file");
    write!(file, "P6\n{} {}\n255\n", img_w, img_h).unwrap();
    file.write_all(&pixels).unwrap();

    println!("Map: {} ({}x{}, seed {})", map_def.name, w, h, seed);
    println!(
        "Spawns: {:?}",
        map_def
            .spawn_points
            .iter()
            .map(|s| s.pos)
            .collect::<Vec<_>>()
    );
    println!("Resources: {}", map_def.resources.len());
    println!("Camps: {}", map_def.neutral_camps.len());
    println!("Written to: {}", output_path);

    // Also output terrain stats
    let mut counts = [0u32; 15];
    for &(t, _) in &map_def.tiles {
        if (t as usize) < counts.len() {
            counts[t as usize] += 1;
        }
    }
    let total = (w * h) as f32;
    println!("\nTerrain distribution:");
    for t in TerrainType::ALL {
        let c = counts[t as usize];
        if c > 0 {
            println!(
                "  {:10}: {:4} ({:.1}%)",
                format!("{:?}", t),
                c,
                c as f32 / total * 100.0
            );
        }
    }
}
