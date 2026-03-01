use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::components::UnitKind;

/// Resource holding unit sprite image handles (art from disk or procedural fallback).
#[derive(Resource)]
pub struct UnitSprites {
    /// One image handle per UnitKind (indexed by kind_index).
    pub sprites: [Handle<Image>; 20],
    /// True if at least one sprite was loaded from a PNG file on disk.
    pub art_loaded: bool,
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
        _ => 0,
    }
}

/// Base sprite dimensions per unit kind (drawing resolution).
fn draw_size(kind: UnitKind) -> (usize, usize) {
    match kind {
        // Cat units
        UnitKind::Pawdler => (16, 16),
        UnitKind::Nuisance => (14, 14),
        UnitKind::Chonk => (24, 24),
        UnitKind::FlyingFox => (18, 18),
        UnitKind::Hisser => (18, 18),
        UnitKind::Yowler => (18, 18),
        UnitKind::Mouser => (14, 14),
        UnitKind::Catnapper => (20, 16),
        UnitKind::FerretSapper => (16, 18),
        UnitKind::MechCommander => (28, 28),
        // Clawed (mice) units — slightly smaller than cats
        UnitKind::Nibblet => (14, 14),
        UnitKind::Swarmer => (12, 12),
        UnitKind::Gnawer => (14, 16),
        UnitKind::Shrieker => (14, 14),
        UnitKind::Tunneler => (16, 14),
        UnitKind::Sparks => (14, 14),
        UnitKind::Quillback => (20, 20),
        UnitKind::Whiskerwitch => (16, 16),
        UnitKind::Plaguetail => (16, 16),
        UnitKind::WarrenMarshal => (22, 22),
        _ => (16, 16),
    }
}

/// Final sprite dimensions (2× draw size for crisp close-up zoom).
/// Display size is controlled by `unit_scale()` in setup.rs via Transform.
fn sprite_size(kind: UnitKind) -> (usize, usize) {
    let (w, h) = draw_size(kind);
    (w * 2, h * 2)
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
        _ => "pawdler",
    }
}

/// Return the asset path for a unit's idle sprite PNG (relative to `assets/`).
pub fn sprite_file_path(kind: UnitKind) -> String {
    let name = unit_slug(kind);
    format!("sprites/units/{name}_idle.png")
}

/// All unit kinds in canonical order (cats 0-9, mice 10-19).
pub const ALL_KINDS: [UnitKind; 20] = [
    // Cat (catGPT) units
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
    // Clawed (mice) units
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
];

/// Generate unit sprite handles at startup. Tries to load PNGs from disk first,
/// falls back to procedural generation for any missing sprites.
pub fn generate_unit_sprites(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(20);
    let mut any_art_loaded = false;

    for kind in ALL_KINDS {
        let asset_path = sprite_file_path(kind);

        let use_disk = super::asset_exists_on_disk(&asset_path);

        if use_disk {
            handles.push(asset_server.load(asset_path));
            any_art_loaded = true;
        } else {
            let img = generate_unit_image(kind);
            handles.push(images.add(img));
        }
    }

    commands.insert_resource(UnitSprites {
        sprites: handles.try_into().expect("exactly 20 unit sprites"),
        art_loaded: any_art_loaded,
    });
}

/// Generate a unit sprite image. Drawn at 1× resolution in neutral gray with dark outline,
/// then upscaled 2× with nearest-neighbor for crisp close-up zoom.
/// Team color is applied as a tint via Sprite::color.
fn generate_unit_image(kind: UnitKind) -> Image {
    let (dw, dh) = draw_size(kind);
    let (fw, fh) = sprite_size(kind);
    let mut draw_data = vec![0u8; dw * dh * 4];

    // Draw the silhouette at 1× resolution
    match kind {
        // Cat units
        UnitKind::Pawdler => draw_pawdler(&mut draw_data, dw, dh),
        UnitKind::Nuisance => draw_nuisance(&mut draw_data, dw, dh),
        UnitKind::Chonk => draw_chonk(&mut draw_data, dw, dh),
        UnitKind::FlyingFox => draw_flying_fox(&mut draw_data, dw, dh),
        UnitKind::Hisser => draw_hisser(&mut draw_data, dw, dh),
        UnitKind::Yowler => draw_yowler(&mut draw_data, dw, dh),
        UnitKind::Mouser => draw_mouser(&mut draw_data, dw, dh),
        UnitKind::Catnapper => draw_catnapper(&mut draw_data, dw, dh),
        UnitKind::FerretSapper => draw_ferret_sapper(&mut draw_data, dw, dh),
        UnitKind::MechCommander => draw_mech_commander(&mut draw_data, dw, dh),
        // Clawed (mice) units
        UnitKind::Nibblet => draw_nibblet(&mut draw_data, dw, dh),
        UnitKind::Swarmer => draw_swarmer(&mut draw_data, dw, dh),
        UnitKind::Gnawer => draw_gnawer(&mut draw_data, dw, dh),
        UnitKind::Shrieker => draw_shrieker(&mut draw_data, dw, dh),
        UnitKind::Tunneler => draw_tunneler(&mut draw_data, dw, dh),
        UnitKind::Sparks => draw_sparks(&mut draw_data, dw, dh),
        UnitKind::Quillback => draw_quillback(&mut draw_data, dw, dh),
        UnitKind::Whiskerwitch => draw_whiskerwitch(&mut draw_data, dw, dh),
        UnitKind::Plaguetail => draw_plaguetail(&mut draw_data, dw, dh),
        UnitKind::WarrenMarshal => draw_warren_marshal(&mut draw_data, dw, dh),
        _ => draw_pawdler(&mut draw_data, dw, dh),
    }

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

// --- Pixel drawing helpers ---

fn set_pixel(data: &mut [u8], w: usize, h: usize, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        let idx = (y as usize * w + x as usize) * 4;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
        data[idx + 3] = a;
    }
}

/// Fill a circle at center (cx, cy) with radius r.
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

/// Draw an outlined circle (body + dark outline ring).
fn draw_body_circle(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, radius: f32) {
    // Outline (dark border)
    fill_circle(data, w, h, cx, cy, radius + 1.0, 40, 40, 40);
    // Body (neutral gray for team tint)
    fill_circle(data, w, h, cx, cy, radius, 180, 180, 180);
}

/// Draw cat ears (two triangles above body).
fn draw_ears(data: &mut [u8], w: usize, h: usize, cx: f32, top_y: f32, ear_w: f32) {
    // Left ear
    for dy in 0..5 {
        let span = ((5 - dy) as f32 * ear_w / 5.0) as i32;
        let base_x = (cx - ear_w * 1.5) as i32;
        let y = (top_y - dy as f32) as i32;
        for dx in 0..span {
            set_pixel(data, w, h, base_x + dx, y, 200, 200, 200, 255);
        }
    }
    // Right ear
    for dy in 0..5 {
        let span = ((5 - dy) as f32 * ear_w / 5.0) as i32;
        let base_x = (cx + ear_w * 0.5) as i32;
        let y = (top_y - dy as f32) as i32;
        for dx in 0..span {
            set_pixel(data, w, h, base_x + dx, y, 200, 200, 200, 255);
        }
    }
}

/// Draw simple eyes (two dark dots).
fn draw_eyes(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spacing: f32) {
    // Eye whites
    fill_circle(data, w, h, cx - spacing, cy, 2.0, 240, 240, 245);
    fill_circle(data, w, h, cx + spacing, cy, 2.0, 240, 240, 245);
    // Pupils
    fill_circle(data, w, h, cx - spacing, cy + 0.5, 1.0, 20, 20, 30);
    fill_circle(data, w, h, cx + spacing, cy + 0.5, 1.0, 20, 20, 30);
}

// --- Individual unit drawing functions ---

fn draw_pawdler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Small cat body
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.0);
    draw_ears(data, w, h, cx, cy - 3.0, 3.0);
    draw_eyes(data, w, h, cx, cy, 2.0);
    // Hard hat (yellow rectangle on top)
    for y in (cy as i32 - 5)..=(cy as i32 - 3) {
        for x in (cx as i32 - 4)..=(cx as i32 + 4) {
            set_pixel(data, w, h, x, y, 220, 200, 60, 255);
        }
    }
    // Pickaxe (small diagonal line)
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 + 4 + i, cy as i32 + i, 140, 130, 120, 255);
    }
}

fn draw_nuisance(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Spiky crouched cat
    draw_body_circle(data, w, h, cx, cy + 1.0, 4.5);
    draw_ears(data, w, h, cx, cy - 2.5, 2.5);
    draw_eyes(data, w, h, cx, cy, 1.5);
    // Wide eyes (extra white)
    fill_circle(data, w, h, cx - 1.5, cy, 1.5, 255, 255, 255);
    fill_circle(data, w, h, cx + 1.5, cy, 1.5, 255, 255, 255);
    fill_circle(data, w, h, cx - 1.5, cy + 0.5, 0.8, 20, 20, 30);
    fill_circle(data, w, h, cx + 1.5, cy + 0.5, 0.8, 20, 20, 30);
    // Spiky fur on top
    for i in 0..3 {
        let sx = cx as i32 - 2 + i * 2;
        set_pixel(data, w, h, sx, cy as i32 - 6, 180, 180, 180, 255);
    }
}

fn draw_chonk(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Fat round cat (loaf shape — wider ellipse)
    // Outline
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 10.0;
            let dy = (py as f32 - cy) / 8.0;
            if dx * dx + dy * dy <= 1.2 {
                set_pixel(data, w, h, px as i32, py as i32, 40, 40, 40, 255);
            }
        }
    }
    // Body fill
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 9.0;
            let dy = (py as f32 - cy) / 7.0;
            if dx * dx + dy * dy <= 1.0 {
                set_pixel(data, w, h, px as i32, py as i32, 180, 180, 180, 255);
            }
        }
    }
    draw_ears(data, w, h, cx, cy - 5.0, 4.0);
    draw_eyes(data, w, h, cx, cy - 1.0, 3.0);
}

fn draw_flying_fox(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Small body
    draw_body_circle(data, w, h, cx, cy, 4.0);
    draw_eyes(data, w, h, cx, cy - 0.5, 1.5);
    // Wings spread (two triangular shapes)
    for i in 0..6 {
        let wing_y = cy as i32 - 2 + i / 2;
        // Left wing
        set_pixel(data, w, h, cx as i32 - 5 - i, wing_y, 160, 160, 160, 255);
        set_pixel(data, w, h, cx as i32 - 5 - i, wing_y + 1, 160, 160, 160, 255);
        // Right wing
        set_pixel(data, w, h, cx as i32 + 5 + i, wing_y, 160, 160, 160, 255);
        set_pixel(data, w, h, cx as i32 + 5 + i, wing_y + 1, 160, 160, 160, 255);
    }
}

fn draw_hisser(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Arched cat
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.5);
    draw_ears(data, w, h, cx, cy - 3.5, 3.0);
    draw_eyes(data, w, h, cx, cy, 2.0);
    // Projectile indicator (small diamond to the right)
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 7 + i, cy as i32, 255, 200, 60, 255);
        set_pixel(data, w, h, cx as i32 + 8, cy as i32 - 1 + i, 255, 200, 60, 255);
    }
}

fn draw_yowler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Cat with mouth open
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.5);
    draw_ears(data, w, h, cx, cy - 3.5, 3.0);
    draw_eyes(data, w, h, cx, cy - 0.5, 2.0);
    // Open mouth (pink oval)
    fill_circle(data, w, h, cx, cy + 3.0, 2.0, 220, 130, 140);
    // Sound wave lines
    for i in 0..3 {
        let sx = cx as i32 + 7;
        let sy = cy as i32 - 2 + i * 2;
        set_pixel(data, w, h, sx, sy, 200, 200, 220, 200);
        set_pixel(data, w, h, sx + 1, sy, 200, 200, 220, 150);
    }
}

fn draw_mouser(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Sleek dark cat with thin outline
    fill_circle(data, w, h, cx, cy + 0.5, 5.0, 50, 50, 50); // Dark outline
    fill_circle(data, w, h, cx, cy + 0.5, 4.0, 120, 120, 125); // Darker body
    draw_ears(data, w, h, cx, cy - 2.5, 2.5);
    // Subtle eyes (nearly invisible)
    fill_circle(data, w, h, cx - 1.5, cy, 1.0, 80, 200, 80);
    fill_circle(data, w, h, cx + 1.5, cy, 1.0, 80, 200, 80);
}

fn draw_catnapper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Curled sleeping cat (wider than tall)
    // Outline
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 8.0;
            let dy = (py as f32 - cy) / 5.5;
            if dx * dx + dy * dy <= 1.15 {
                set_pixel(data, w, h, px as i32, py as i32, 40, 40, 40, 255);
            }
        }
    }
    // Body
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 7.0;
            let dy = (py as f32 - cy) / 4.5;
            if dx * dx + dy * dy <= 1.0 {
                set_pixel(data, w, h, px as i32, py as i32, 180, 180, 180, 255);
            }
        }
    }
    // Zzz dots above
    set_pixel(data, w, h, cx as i32 + 3, 1, 200, 200, 220, 200);
    set_pixel(data, w, h, cx as i32 + 5, 0, 200, 200, 220, 180);
    set_pixel(data, w, h, cx as i32 + 7, 1, 200, 200, 220, 160);
}

fn draw_ferret_sapper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Wiry ferret body (tall, narrow)
    // Outline
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 5.0;
            let dy = (py as f32 - cy) / 7.0;
            if dx * dx + dy * dy <= 1.15 {
                set_pixel(data, w, h, px as i32, py as i32, 40, 40, 40, 255);
            }
        }
    }
    // Body
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 4.0;
            let dy = (py as f32 - cy) / 6.0;
            if dx * dx + dy * dy <= 1.0 {
                set_pixel(data, w, h, px as i32, py as i32, 175, 160, 145, 255);
            }
        }
    }
    // Pointy face
    draw_eyes(data, w, h, cx, cy - 2.0, 1.5);
    // Small bomb shape at bottom
    fill_circle(data, w, h, cx + 3.0, cy + 5.0, 2.0, 60, 60, 60);
    // Fuse spark
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 + 3, 255, 200, 50, 255);
}

fn draw_mech_commander(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let _cy = h as f32 / 2.0;
    // Large mech suit body (blocky)
    // Outer frame (dark)
    for py in 4..h - 2 {
        for px in 4..w - 4 {
            set_pixel(data, w, h, px as i32, py as i32, 60, 60, 70, 255);
        }
    }
    // Inner body (lighter)
    for py in 6..h - 4 {
        for px in 6..w - 6 {
            set_pixel(data, w, h, px as i32, py as i32, 150, 150, 160, 255);
        }
    }
    // Cockpit window (small bright area in upper center)
    for py in 7..11 {
        for px in (cx as usize - 3)..(cx as usize + 3) {
            set_pixel(data, w, h, px as i32, py as i32, 200, 220, 240, 255);
        }
    }
    // Cat silhouette in cockpit
    fill_circle(data, w, h, cx, 9.0, 2.0, 180, 180, 180);
    // Star command icon
    set_pixel(data, w, h, cx as i32, 2, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 - 1, 3, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 + 1, 3, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32, 4, 255, 220, 50, 255);
    // Legs
    for py in (h - 4)..h {
        set_pixel(data, w, h, cx as i32 - 5, py as i32, 80, 80, 90, 255);
        set_pixel(data, w, h, cx as i32 - 4, py as i32, 80, 80, 90, 255);
        set_pixel(data, w, h, cx as i32 + 4, py as i32, 80, 80, 90, 255);
        set_pixel(data, w, h, cx as i32 + 5, py as i32, 80, 80, 90, 255);
    }
}

// --- Mouse (Clawed) drawing functions ---
// Mice use round ears (not pointed) and a long tail. Body is slightly
// pear-shaped. Base color is a warm brown-gray for team tint.

/// Draw round mouse ears (two circles above body).
fn draw_mouse_ears(data: &mut [u8], w: usize, h: usize, cx: f32, top_y: f32, ear_r: f32) {
    fill_circle(data, w, h, cx - ear_r * 1.8, top_y - ear_r, ear_r + 0.5, 40, 40, 40);
    fill_circle(data, w, h, cx - ear_r * 1.8, top_y - ear_r, ear_r, 200, 180, 170);
    fill_circle(data, w, h, cx + ear_r * 1.8, top_y - ear_r, ear_r + 0.5, 40, 40, 40);
    fill_circle(data, w, h, cx + ear_r * 1.8, top_y - ear_r, ear_r, 200, 180, 170);
}

/// Draw a mouse body (outline + fill).
fn draw_mouse_body(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, radius: f32) {
    fill_circle(data, w, h, cx, cy, radius + 1.0, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, radius, 190, 175, 165);
}

/// Draw a curving mouse tail.
fn draw_mouse_tail(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32) {
    for i in 0..5 {
        let tx = cx as i32 + 2 + i;
        let ty = cy as i32 + 2 + (i / 2);
        set_pixel(data, w, h, tx, ty, 160, 140, 130, 255);
    }
}

/// Draw small beady mouse eyes.
fn draw_mouse_eyes(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spacing: f32) {
    fill_circle(data, w, h, cx - spacing, cy, 1.0, 20, 20, 20);
    fill_circle(data, w, h, cx + spacing, cy, 1.0, 20, 20, 20);
}

fn draw_nibblet(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 4.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.5, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    draw_mouse_tail(data, w, h, cx, cy + 2.0);
    // Tiny pickaxe
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 3 + i, cy as i32 + i, 140, 130, 120, 255);
    }
}

fn draw_swarmer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Small, fast mouse
    draw_mouse_body(data, w, h, cx, cy + 0.5, 3.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.0, 1.5);
    draw_mouse_eyes(data, w, h, cx, cy, 1.2);
    draw_mouse_tail(data, w, h, cx, cy + 1.5);
    // Tiny sword slash
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 - 1, 200, 200, 210, 255);
    set_pixel(data, w, h, cx as i32 + 4, cy as i32, 200, 200, 210, 255);
}

fn draw_gnawer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 4.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.0, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    // Prominent front teeth
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 + 3, 240, 240, 220, 255);
    set_pixel(data, w, h, cx as i32, cy as i32 + 3, 240, 240, 220, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 + 4, 240, 240, 220, 255);
    set_pixel(data, w, h, cx as i32, cy as i32 + 4, 240, 240, 220, 255);
}

fn draw_shrieker(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 4.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.5, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    // Open mouth (shrieking)
    fill_circle(data, w, h, cx, cy + 2.5, 1.5, 200, 100, 100);
    // Sound wave dots
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 5, cy as i32 - 1 + i, 220, 200, 180, 180);
    }
}

fn draw_tunneler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Wide, squat mole-like mouse
    // Outline
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 6.5;
            let dy = (py as f32 - cy) / 4.5;
            if dx * dx + dy * dy <= 1.15 {
                set_pixel(data, w, h, px as i32, py as i32, 40, 40, 40, 255);
            }
        }
    }
    // Body
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / 5.5;
            let dy = (py as f32 - cy) / 3.5;
            if dx * dx + dy * dy <= 1.0 {
                set_pixel(data, w, h, px as i32, py as i32, 170, 155, 140, 255);
            }
        }
    }
    draw_mouse_ears(data, w, h, cx, cy - 2.5, 1.5);
    draw_mouse_eyes(data, w, h, cx, cy - 0.5, 1.5);
    // Digging claws
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 - 4 + i, cy as i32 + 4, 120, 110, 100, 255);
        set_pixel(data, w, h, cx as i32 + 2 + i, cy as i32 + 4, 120, 110, 100, 255);
    }
}

fn draw_sparks(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 0.5, 4.0);
    draw_mouse_ears(data, w, h, cx, cy - 2.5, 1.8);
    draw_mouse_eyes(data, w, h, cx, cy, 1.2);
    // Electric sparks (yellow dots around body)
    set_pixel(data, w, h, cx as i32 - 4, cy as i32 - 2, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 - 1, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 + 3, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 - 3, cy as i32 + 2, 255, 255, 100, 255);
}

fn draw_quillback(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large, armored mouse with spines
    fill_circle(data, w, h, cx, cy, 8.0, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, 7.0, 190, 175, 165);
    draw_mouse_ears(data, w, h, cx, cy - 5.0, 2.5);
    draw_mouse_eyes(data, w, h, cx, cy - 1.0, 2.0);
    // Quill spines on back
    for i in 0..5 {
        let sx = cx as i32 - 4 + i * 2;
        set_pixel(data, w, h, sx, cy as i32 - 7, 160, 140, 120, 255);
        set_pixel(data, w, h, sx, cy as i32 - 8, 160, 140, 120, 255);
    }
}

fn draw_whiskerwitch(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 5.0);
    draw_mouse_ears(data, w, h, cx, cy - 3.0, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    // Long whiskers
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 5 - i, cy as i32 + 1 + (i / 2), 140, 130, 120, 200);
        set_pixel(data, w, h, cx as i32 + 5 + i, cy as i32 + 1 + (i / 2), 140, 130, 120, 200);
    }
    // Magic glow (purple tint at paws)
    fill_circle(data, w, h, cx - 3.0, cy + 4.0, 1.5, 160, 100, 200);
    fill_circle(data, w, h, cx + 3.0, cy + 4.0, 1.5, 160, 100, 200);
}

fn draw_plaguetail(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 5.0);
    draw_mouse_ears(data, w, h, cx, cy - 3.0, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    // Sickly green-tinged tail (thicker, discolored)
    for i in 0..6 {
        let tx = cx as i32 + 2 + i;
        let ty = cy as i32 + 3 + (i / 2);
        set_pixel(data, w, h, tx, ty, 120, 180, 80, 255);
        set_pixel(data, w, h, tx, ty + 1, 120, 180, 80, 200);
    }
    // Toxic cloud dots
    fill_circle(data, w, h, cx + 1.0, cy - 4.0, 1.5, 100, 160, 60);
    fill_circle(data, w, h, cx - 2.0, cy - 5.0, 1.0, 100, 160, 60);
}

fn draw_warren_marshal(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large commanding mouse
    fill_circle(data, w, h, cx, cy, 8.5, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, 7.5, 190, 175, 165);
    draw_mouse_ears(data, w, h, cx, cy - 5.5, 2.5);
    draw_mouse_eyes(data, w, h, cx, cy - 1.0, 2.0);
    // Command star on forehead
    set_pixel(data, w, h, cx as i32, cy as i32 - 5, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 - 4, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 - 4, 255, 220, 50, 255);
    // Banner/standard behind
    for py in (cy as i32 - 8)..(cy as i32 - 3) {
        set_pixel(data, w, h, cx as i32 + 7, py, 200, 60, 60, 255);
    }
    set_pixel(data, w, h, cx as i32 + 7, cy as i32 - 9, 140, 140, 140, 255);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_file_paths_match_catalog() {
        // Cat unit paths
        assert_eq!(sprite_file_path(UnitKind::Pawdler), "sprites/units/pawdler_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Nuisance), "sprites/units/nuisance_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Chonk), "sprites/units/chonk_idle.png");
        assert_eq!(sprite_file_path(UnitKind::FlyingFox), "sprites/units/flying_fox_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Hisser), "sprites/units/hisser_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Yowler), "sprites/units/yowler_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Mouser), "sprites/units/mouser_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Catnapper), "sprites/units/catnapper_idle.png");
        assert_eq!(sprite_file_path(UnitKind::FerretSapper), "sprites/units/ferret_sapper_idle.png");
        assert_eq!(sprite_file_path(UnitKind::MechCommander), "sprites/units/mech_commander_idle.png");
        // Clawed (mice) unit paths
        assert_eq!(sprite_file_path(UnitKind::Nibblet), "sprites/units/nibblet_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Swarmer), "sprites/units/swarmer_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Gnawer), "sprites/units/gnawer_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Shrieker), "sprites/units/shrieker_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Tunneler), "sprites/units/tunneler_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Sparks), "sprites/units/sparks_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Quillback), "sprites/units/quillback_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Whiskerwitch), "sprites/units/whiskerwitch_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Plaguetail), "sprites/units/plaguetail_idle.png");
        assert_eq!(sprite_file_path(UnitKind::WarrenMarshal), "sprites/units/warren_marshal_idle.png");
    }

    #[test]
    fn all_kinds_have_sprite_paths() {
        for kind in ALL_KINDS {
            let path = sprite_file_path(kind);
            assert!(path.starts_with("sprites/units/"), "Path should be under sprites/units/: {path}");
            assert!(path.ends_with("_idle.png"), "Path should end with _idle.png: {path}");
        }
    }

    #[test]
    fn kind_index_covers_all_kinds() {
        for (i, kind) in ALL_KINDS.iter().enumerate() {
            assert_eq!(kind_index(*kind), i, "kind_index mismatch for {kind:?}");
        }
    }

    #[test]
    fn all_kinds_constant_has_twenty_entries() {
        assert_eq!(ALL_KINDS.len(), 20);
    }

    /// Cat units (0-9) should have art files on disk.
    /// Clawed (mice) units (10-19) use procedural fallbacks for now.
    #[test]
    fn cat_sprite_files_exist_on_disk() {
        let asset_root = std::path::Path::new("../../assets");
        for kind in &ALL_KINDS[..10] {
            let asset_path = sprite_file_path(*kind);
            let full_path = asset_root.join(&asset_path);
            assert!(
                full_path.exists(),
                "Sprite file missing for {kind:?}: {}",
                full_path.display()
            );
        }
    }

    #[test]
    fn clawed_unit_slugs_are_valid() {
        let clawed_kinds = &ALL_KINDS[10..20];
        for kind in clawed_kinds {
            let slug = unit_slug(*kind);
            assert!(!slug.is_empty(), "Empty slug for {kind:?}");
            assert_ne!(slug, "pawdler", "Clawed unit {kind:?} should not fall through to pawdler");
        }
    }

    #[test]
    fn procedural_images_are_nonzero() {
        for kind in ALL_KINDS {
            let img = generate_unit_image(kind);
            let data = img.data.as_ref().expect("Image should have data");
            let has_nonzero = data.iter().any(|&b| b != 0);
            assert!(has_nonzero, "Procedural image for {kind:?} is all zeros");
        }
    }
}
