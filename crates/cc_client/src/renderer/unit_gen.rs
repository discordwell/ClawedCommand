use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::components::UnitKind;

/// Resource holding procedurally generated unit sprite images.
#[derive(Resource)]
pub struct UnitSprites {
    /// One image handle per UnitKind (indexed by kind_index).
    pub sprites: [Handle<Image>; 10],
}

/// Map UnitKind to array index.
pub fn kind_index(kind: UnitKind) -> usize {
    match kind {
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
    }
}

/// Base sprite dimensions per unit kind (drawing resolution).
fn draw_size(kind: UnitKind) -> (usize, usize) {
    match kind {
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
    }
}

/// Final sprite dimensions (2× draw size for crisp close-up zoom).
/// Display size is controlled by `unit_scale()` in setup.rs via Transform.
fn sprite_size(kind: UnitKind) -> (usize, usize) {
    let (w, h) = draw_size(kind);
    (w * 2, h * 2)
}

/// Generate unit sprite images at startup.
pub fn generate_unit_sprites(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let kinds = [
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
    ];

    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(10);
    for kind in kinds {
        let img = generate_unit_image(kind);
        handles.push(images.add(img));
    }

    commands.insert_resource(UnitSprites {
        sprites: [
            handles[0].clone(),
            handles[1].clone(),
            handles[2].clone(),
            handles[3].clone(),
            handles[4].clone(),
            handles[5].clone(),
            handles[6].clone(),
            handles[7].clone(),
            handles[8].clone(),
            handles[9].clone(),
        ],
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
