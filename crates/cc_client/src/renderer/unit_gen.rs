use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::components::UnitKind;

/// Resource holding unit sprite image handles (art from disk or procedural fallback).
#[derive(Resource)]
pub struct UnitSprites {
    /// One image handle per UnitKind (indexed by kind_index).
    pub sprites: [Handle<Image>; 60],
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
        // Murder (corvids) units: 20-29
        UnitKind::MurderScrounger => 20,
        UnitKind::Sentinel => 21,
        UnitKind::Rookclaw => 22,
        UnitKind::Magpike => 23,
        UnitKind::Magpyre => 24,
        UnitKind::Jaycaller => 25,
        UnitKind::Jayflicker => 26,
        UnitKind::Dusktalon => 27,
        UnitKind::Hootseer => 28,
        UnitKind::CorvusRex => 29,
        // Seekers (badgers) units: 30-39
        UnitKind::Delver => 30,
        UnitKind::Ironhide => 31,
        UnitKind::Cragback => 32,
        UnitKind::Warden => 33,
        UnitKind::Sapjaw => 34,
        UnitKind::Wardenmother => 35,
        UnitKind::SeekerTunneler => 36,
        UnitKind::Embermaw => 37,
        UnitKind::Dustclaw => 38,
        UnitKind::Gutripper => 39,
        // Croak (axolotls) units: 40-49
        UnitKind::Ponderer => 40,
        UnitKind::Regeneron => 41,
        UnitKind::Broodmother => 42,
        UnitKind::Gulper => 43,
        UnitKind::Eftsaber => 44,
        UnitKind::Croaker => 45,
        UnitKind::Leapfrog => 46,
        UnitKind::Shellwarden => 47,
        UnitKind::Bogwhisper => 48,
        UnitKind::MurkCommander => 49,
        // LLAMA (raccoons) units: 50-59
        UnitKind::Scrounger => 50,
        UnitKind::Bandit => 51,
        UnitKind::HeapTitan => 52,
        UnitKind::GlitchRat => 53,
        UnitKind::PatchPossum => 54,
        UnitKind::GreaseMonkey => 55,
        UnitKind::DeadDropUnit => 56,
        UnitKind::Wrecker => 57,
        UnitKind::DumpsterDiver => 58,
        UnitKind::JunkyardKing => 59,
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
        // Clawed (mice) units
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
        // Murder (corvids) — aerial, fragile
        UnitKind::MurderScrounger => (14, 14),
        UnitKind::Sentinel => (14, 16),
        UnitKind::Rookclaw => (16, 14),
        UnitKind::Magpike => (14, 14),
        UnitKind::Magpyre => (14, 14),
        UnitKind::Jaycaller => (16, 16),
        UnitKind::Jayflicker => (16, 16),
        UnitKind::Dusktalon => (16, 16),
        UnitKind::Hootseer => (18, 18),
        UnitKind::CorvusRex => (24, 24),
        // Seekers (badgers) — heavy, slow
        UnitKind::Delver => (16, 16),
        UnitKind::Ironhide => (20, 20),
        UnitKind::Cragback => (22, 22),
        UnitKind::Warden => (18, 18),
        UnitKind::Sapjaw => (18, 18),
        UnitKind::Wardenmother => (22, 22),
        UnitKind::SeekerTunneler => (16, 14),
        UnitKind::Embermaw => (18, 18),
        UnitKind::Dustclaw => (14, 14),
        UnitKind::Gutripper => (20, 20),
        // Croak (axolotls) — medium, regenerating
        UnitKind::Ponderer => (16, 16),
        UnitKind::Regeneron => (14, 14),
        UnitKind::Broodmother => (18, 18),
        UnitKind::Gulper => (20, 18),
        UnitKind::Eftsaber => (14, 16),
        UnitKind::Croaker => (16, 16),
        UnitKind::Leapfrog => (16, 14),
        UnitKind::Shellwarden => (18, 18),
        UnitKind::Bogwhisper => (16, 16),
        UnitKind::MurkCommander => (24, 24),
        // LLAMA (raccoons) — medium, scrappy
        UnitKind::Scrounger => (14, 14),
        UnitKind::Bandit => (14, 14),
        UnitKind::HeapTitan => (22, 22),
        UnitKind::GlitchRat => (12, 14),
        UnitKind::PatchPossum => (14, 16),
        UnitKind::GreaseMonkey => (16, 16),
        UnitKind::DeadDropUnit => (14, 16),
        UnitKind::Wrecker => (18, 18),
        UnitKind::DumpsterDiver => (16, 16),
        UnitKind::JunkyardKing => (24, 24),
    }
}

/// Final sprite dimensions (2x draw size for crisp close-up zoom).
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
        // Murder (corvids)
        UnitKind::MurderScrounger => "murder_scrounger",
        UnitKind::Sentinel => "sentinel",
        UnitKind::Rookclaw => "rookclaw",
        UnitKind::Magpike => "magpike",
        UnitKind::Magpyre => "magpyre",
        UnitKind::Jaycaller => "jaycaller",
        UnitKind::Jayflicker => "jayflicker",
        UnitKind::Dusktalon => "dusktalon",
        UnitKind::Hootseer => "hootseer",
        UnitKind::CorvusRex => "corvus_rex",
        // Seekers (badgers)
        UnitKind::Delver => "delver",
        UnitKind::Ironhide => "ironhide",
        UnitKind::Cragback => "cragback",
        UnitKind::Warden => "warden",
        UnitKind::Sapjaw => "sapjaw",
        UnitKind::Wardenmother => "wardenmother",
        UnitKind::SeekerTunneler => "seeker_tunneler",
        UnitKind::Embermaw => "embermaw",
        UnitKind::Dustclaw => "dustclaw",
        UnitKind::Gutripper => "gutripper",
        // Croak (axolotls)
        UnitKind::Ponderer => "ponderer",
        UnitKind::Regeneron => "regeneron",
        UnitKind::Broodmother => "broodmother",
        UnitKind::Gulper => "gulper",
        UnitKind::Eftsaber => "eftsaber",
        UnitKind::Croaker => "croaker",
        UnitKind::Leapfrog => "leapfrog",
        UnitKind::Shellwarden => "shellwarden",
        UnitKind::Bogwhisper => "bogwhisper",
        UnitKind::MurkCommander => "murk_commander",
        // LLAMA (raccoons)
        UnitKind::Scrounger => "scrounger",
        UnitKind::Bandit => "bandit",
        UnitKind::HeapTitan => "heap_titan",
        UnitKind::GlitchRat => "glitch_rat",
        UnitKind::PatchPossum => "patch_possum",
        UnitKind::GreaseMonkey => "grease_monkey",
        UnitKind::DeadDropUnit => "dead_drop_unit",
        UnitKind::Wrecker => "wrecker",
        UnitKind::DumpsterDiver => "dumpster_diver",
        UnitKind::JunkyardKing => "junkyard_king",
    }
}

/// Return the asset path for a unit's idle sprite PNG (relative to `assets/`).
pub fn sprite_file_path(kind: UnitKind) -> String {
    let name = unit_slug(kind);
    format!("sprites/units/{name}_idle.png")
}

/// All unit kinds in canonical order (cats 0-9, clawed 10-19, murder 20-29,
/// seekers 30-39, croak 40-49, llama 50-59).
pub const ALL_KINDS: [UnitKind; 60] = [
    // Cat (catGPT) units: 0-9
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
    // Clawed (mice) units: 10-19
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
    // Murder (corvids) units: 20-29
    UnitKind::MurderScrounger,
    UnitKind::Sentinel,
    UnitKind::Rookclaw,
    UnitKind::Magpike,
    UnitKind::Magpyre,
    UnitKind::Jaycaller,
    UnitKind::Jayflicker,
    UnitKind::Dusktalon,
    UnitKind::Hootseer,
    UnitKind::CorvusRex,
    // Seekers (badgers) units: 30-39
    UnitKind::Delver,
    UnitKind::Ironhide,
    UnitKind::Cragback,
    UnitKind::Warden,
    UnitKind::Sapjaw,
    UnitKind::Wardenmother,
    UnitKind::SeekerTunneler,
    UnitKind::Embermaw,
    UnitKind::Dustclaw,
    UnitKind::Gutripper,
    // Croak (axolotls) units: 40-49
    UnitKind::Ponderer,
    UnitKind::Regeneron,
    UnitKind::Broodmother,
    UnitKind::Gulper,
    UnitKind::Eftsaber,
    UnitKind::Croaker,
    UnitKind::Leapfrog,
    UnitKind::Shellwarden,
    UnitKind::Bogwhisper,
    UnitKind::MurkCommander,
    // LLAMA (raccoons) units: 50-59
    UnitKind::Scrounger,
    UnitKind::Bandit,
    UnitKind::HeapTitan,
    UnitKind::GlitchRat,
    UnitKind::PatchPossum,
    UnitKind::GreaseMonkey,
    UnitKind::DeadDropUnit,
    UnitKind::Wrecker,
    UnitKind::DumpsterDiver,
    UnitKind::JunkyardKing,
];

/// Generate unit sprite handles at startup. Tries to load PNGs from disk first,
/// falls back to procedural generation for any missing sprites.
pub fn generate_unit_sprites(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(60);
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
        sprites: handles.try_into().expect("exactly 60 unit sprites"),
        art_loaded: any_art_loaded,
    });
}

/// Generate a unit sprite image. Drawn at 1x resolution in neutral gray with dark outline,
/// then upscaled 2x with nearest-neighbor for crisp close-up zoom.
/// Team color is applied as a tint via Sprite::color.
fn generate_unit_image(kind: UnitKind) -> Image {
    let (dw, dh) = draw_size(kind);
    let (fw, fh) = sprite_size(kind);
    let mut draw_data = vec![0u8; dw * dh * 4];

    // Draw the silhouette at 1x resolution
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
        // Murder (corvids)
        UnitKind::MurderScrounger => draw_murder_scrounger(&mut draw_data, dw, dh),
        UnitKind::Sentinel => draw_sentinel(&mut draw_data, dw, dh),
        UnitKind::Rookclaw => draw_rookclaw(&mut draw_data, dw, dh),
        UnitKind::Magpike => draw_magpike(&mut draw_data, dw, dh),
        UnitKind::Magpyre => draw_magpyre(&mut draw_data, dw, dh),
        UnitKind::Jaycaller => draw_jaycaller(&mut draw_data, dw, dh),
        UnitKind::Jayflicker => draw_jayflicker(&mut draw_data, dw, dh),
        UnitKind::Dusktalon => draw_dusktalon(&mut draw_data, dw, dh),
        UnitKind::Hootseer => draw_hootseer(&mut draw_data, dw, dh),
        UnitKind::CorvusRex => draw_corvus_rex(&mut draw_data, dw, dh),
        // Seekers (badgers)
        UnitKind::Delver => draw_delver(&mut draw_data, dw, dh),
        UnitKind::Ironhide => draw_ironhide(&mut draw_data, dw, dh),
        UnitKind::Cragback => draw_cragback(&mut draw_data, dw, dh),
        UnitKind::Warden => draw_warden(&mut draw_data, dw, dh),
        UnitKind::Sapjaw => draw_sapjaw(&mut draw_data, dw, dh),
        UnitKind::Wardenmother => draw_wardenmother(&mut draw_data, dw, dh),
        UnitKind::SeekerTunneler => draw_seeker_tunneler(&mut draw_data, dw, dh),
        UnitKind::Embermaw => draw_embermaw(&mut draw_data, dw, dh),
        UnitKind::Dustclaw => draw_dustclaw(&mut draw_data, dw, dh),
        UnitKind::Gutripper => draw_gutripper(&mut draw_data, dw, dh),
        // Croak (axolotls)
        UnitKind::Ponderer => draw_ponderer(&mut draw_data, dw, dh),
        UnitKind::Regeneron => draw_regeneron(&mut draw_data, dw, dh),
        UnitKind::Broodmother => draw_broodmother(&mut draw_data, dw, dh),
        UnitKind::Gulper => draw_gulper(&mut draw_data, dw, dh),
        UnitKind::Eftsaber => draw_eftsaber(&mut draw_data, dw, dh),
        UnitKind::Croaker => draw_croaker(&mut draw_data, dw, dh),
        UnitKind::Leapfrog => draw_leapfrog(&mut draw_data, dw, dh),
        UnitKind::Shellwarden => draw_shellwarden(&mut draw_data, dw, dh),
        UnitKind::Bogwhisper => draw_bogwhisper(&mut draw_data, dw, dh),
        UnitKind::MurkCommander => draw_murk_commander(&mut draw_data, dw, dh),
        // LLAMA (raccoons)
        UnitKind::Scrounger => draw_scrounger(&mut draw_data, dw, dh),
        UnitKind::Bandit => draw_bandit(&mut draw_data, dw, dh),
        UnitKind::HeapTitan => draw_heap_titan(&mut draw_data, dw, dh),
        UnitKind::GlitchRat => draw_glitch_rat(&mut draw_data, dw, dh),
        UnitKind::PatchPossum => draw_patch_possum(&mut draw_data, dw, dh),
        UnitKind::GreaseMonkey => draw_grease_monkey(&mut draw_data, dw, dh),
        UnitKind::DeadDropUnit => draw_dead_drop_unit(&mut draw_data, dw, dh),
        UnitKind::Wrecker => draw_wrecker(&mut draw_data, dw, dh),
        UnitKind::DumpsterDiver => draw_dumpster_diver(&mut draw_data, dw, dh),
        UnitKind::JunkyardKing => draw_junkyard_king(&mut draw_data, dw, dh),
    }

    // Upscale 2x with nearest-neighbor
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

/// Fill an ellipse at center (cx, cy) with semi-axes (rx, ry).
fn fill_ellipse(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, rx: f32, ry: f32, r: u8, g: u8, b: u8) {
    for py in 0..h {
        for px in 0..w {
            let dx = (px as f32 - cx) / rx;
            let dy = (py as f32 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
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

// ===== Cat (catGPT) drawing functions =====

fn draw_pawdler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.0);
    draw_ears(data, w, h, cx, cy - 3.0, 3.0);
    draw_eyes(data, w, h, cx, cy, 2.0);
    // Hard hat (yellow rectangle on top)
    for y in (cy as i32 - 5)..=(cy as i32 - 3) {
        for x in (cx as i32 - 4)..=(cx as i32 + 4) {
            set_pixel(data, w, h, x, y, 220, 200, 60, 255);
        }
    }
    // Pickaxe
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 + 4 + i, cy as i32 + i, 140, 130, 120, 255);
    }
}

fn draw_nuisance(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_body_circle(data, w, h, cx, cy + 1.0, 4.5);
    draw_ears(data, w, h, cx, cy - 2.5, 2.5);
    draw_eyes(data, w, h, cx, cy, 1.5);
    fill_circle(data, w, h, cx - 1.5, cy, 1.5, 255, 255, 255);
    fill_circle(data, w, h, cx + 1.5, cy, 1.5, 255, 255, 255);
    fill_circle(data, w, h, cx - 1.5, cy + 0.5, 0.8, 20, 20, 30);
    fill_circle(data, w, h, cx + 1.5, cy + 0.5, 0.8, 20, 20, 30);
    for i in 0..3 {
        let sx = cx as i32 - 2 + i * 2;
        set_pixel(data, w, h, sx, cy as i32 - 6, 180, 180, 180, 255);
    }
}

fn draw_chonk(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_ellipse(data, w, h, cx, cy, 10.0, 8.0, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy, 9.0, 7.0, 180, 180, 180);
    draw_ears(data, w, h, cx, cy - 5.0, 4.0);
    draw_eyes(data, w, h, cx, cy - 1.0, 3.0);
}

fn draw_flying_fox(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_body_circle(data, w, h, cx, cy, 4.0);
    draw_eyes(data, w, h, cx, cy - 0.5, 1.5);
    for i in 0..6 {
        let wing_y = cy as i32 - 2 + i / 2;
        set_pixel(data, w, h, cx as i32 - 5 - i, wing_y, 160, 160, 160, 255);
        set_pixel(data, w, h, cx as i32 - 5 - i, wing_y + 1, 160, 160, 160, 255);
        set_pixel(data, w, h, cx as i32 + 5 + i, wing_y, 160, 160, 160, 255);
        set_pixel(data, w, h, cx as i32 + 5 + i, wing_y + 1, 160, 160, 160, 255);
    }
}

fn draw_hisser(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.5);
    draw_ears(data, w, h, cx, cy - 3.5, 3.0);
    draw_eyes(data, w, h, cx, cy, 2.0);
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 7 + i, cy as i32, 255, 200, 60, 255);
        set_pixel(data, w, h, cx as i32 + 8, cy as i32 - 1 + i, 255, 200, 60, 255);
    }
}

fn draw_yowler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_body_circle(data, w, h, cx, cy + 1.0, 5.5);
    draw_ears(data, w, h, cx, cy - 3.5, 3.0);
    draw_eyes(data, w, h, cx, cy - 0.5, 2.0);
    fill_circle(data, w, h, cx, cy + 3.0, 2.0, 220, 130, 140);
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
    fill_circle(data, w, h, cx, cy + 0.5, 5.0, 50, 50, 50);
    fill_circle(data, w, h, cx, cy + 0.5, 4.0, 120, 120, 125);
    draw_ears(data, w, h, cx, cy - 2.5, 2.5);
    fill_circle(data, w, h, cx - 1.5, cy, 1.0, 80, 200, 80);
    fill_circle(data, w, h, cx + 1.5, cy, 1.0, 80, 200, 80);
}

fn draw_catnapper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_ellipse(data, w, h, cx, cy, 8.0, 5.5, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy, 7.0, 4.5, 180, 180, 180);
    set_pixel(data, w, h, cx as i32 + 3, 1, 200, 200, 220, 200);
    set_pixel(data, w, h, cx as i32 + 5, 0, 200, 200, 220, 180);
    set_pixel(data, w, h, cx as i32 + 7, 1, 200, 200, 220, 160);
}

fn draw_ferret_sapper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_ellipse(data, w, h, cx, cy, 5.0, 7.0, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy, 4.0, 6.0, 175, 160, 145);
    draw_eyes(data, w, h, cx, cy - 2.0, 1.5);
    fill_circle(data, w, h, cx + 3.0, cy + 5.0, 2.0, 60, 60, 60);
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 + 3, 255, 200, 50, 255);
}

fn draw_mech_commander(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let _cy = h as f32 / 2.0;
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
    // Cockpit window
    for py in 7..11 {
        for px in (cx as usize - 3)..(cx as usize + 3) {
            set_pixel(data, w, h, px as i32, py as i32, 200, 220, 240, 255);
        }
    }
    fill_circle(data, w, h, cx, 9.0, 2.0, 180, 180, 180);
    // Star
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

// ===== Clawed (Mice) drawing functions =====

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
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 3 + i, cy as i32 + i, 140, 130, 120, 255);
    }
}

fn draw_swarmer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 0.5, 3.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.0, 1.5);
    draw_mouse_eyes(data, w, h, cx, cy, 1.2);
    draw_mouse_tail(data, w, h, cx, cy + 1.5);
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 - 1, 200, 200, 210, 255);
    set_pixel(data, w, h, cx as i32 + 4, cy as i32, 200, 200, 210, 255);
}

fn draw_gnawer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 4.5);
    draw_mouse_ears(data, w, h, cx, cy - 2.0, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
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
    fill_circle(data, w, h, cx, cy + 2.5, 1.5, 200, 100, 100);
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 5, cy as i32 - 1 + i, 220, 200, 180, 180);
    }
}

fn draw_tunneler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_ellipse(data, w, h, cx, cy, 6.5, 4.5, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy, 5.5, 3.5, 170, 155, 140);
    draw_mouse_ears(data, w, h, cx, cy - 2.5, 1.5);
    draw_mouse_eyes(data, w, h, cx, cy - 0.5, 1.5);
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
    set_pixel(data, w, h, cx as i32 - 4, cy as i32 - 2, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 - 1, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 + 3, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 - 3, cy as i32 + 2, 255, 255, 100, 255);
}

fn draw_quillback(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_circle(data, w, h, cx, cy, 8.0, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, 7.0, 190, 175, 165);
    draw_mouse_ears(data, w, h, cx, cy - 5.0, 2.5);
    draw_mouse_eyes(data, w, h, cx, cy - 1.0, 2.0);
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
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 5 - i, cy as i32 + 1 + (i / 2), 140, 130, 120, 200);
        set_pixel(data, w, h, cx as i32 + 5 + i, cy as i32 + 1 + (i / 2), 140, 130, 120, 200);
    }
    fill_circle(data, w, h, cx - 3.0, cy + 4.0, 1.5, 160, 100, 200);
    fill_circle(data, w, h, cx + 3.0, cy + 4.0, 1.5, 160, 100, 200);
}

fn draw_plaguetail(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_mouse_body(data, w, h, cx, cy + 1.0, 5.0);
    draw_mouse_ears(data, w, h, cx, cy - 3.0, 2.0);
    draw_mouse_eyes(data, w, h, cx, cy, 1.5);
    for i in 0..6 {
        let tx = cx as i32 + 2 + i;
        let ty = cy as i32 + 3 + (i / 2);
        set_pixel(data, w, h, tx, ty, 120, 180, 80, 255);
        set_pixel(data, w, h, tx, ty + 1, 120, 180, 80, 200);
    }
    fill_circle(data, w, h, cx + 1.0, cy - 4.0, 1.5, 100, 160, 60);
    fill_circle(data, w, h, cx - 2.0, cy - 5.0, 1.0, 100, 160, 60);
}

fn draw_warren_marshal(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    fill_circle(data, w, h, cx, cy, 8.5, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, 7.5, 190, 175, 165);
    draw_mouse_ears(data, w, h, cx, cy - 5.5, 2.5);
    draw_mouse_eyes(data, w, h, cx, cy - 1.0, 2.0);
    set_pixel(data, w, h, cx as i32, cy as i32 - 5, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 - 4, 255, 220, 50, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 - 4, 255, 220, 50, 255);
    for py in (cy as i32 - 8)..(cy as i32 - 3) {
        set_pixel(data, w, h, cx as i32 + 7, py, 200, 60, 60, 255);
    }
    set_pixel(data, w, h, cx as i32 + 7, cy as i32 - 9, 140, 140, 140, 255);
}

// ===== Murder (Corvid) drawing functions =====
// Corvids have sleek, angular bodies with pointed beaks and wing shapes.
// Base body color is dark gray (tinted by team color).

/// Draw a bird body (oval with dark outline, dark-gray fill).
fn draw_bird_body(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, rx: f32, ry: f32) {
    fill_ellipse(data, w, h, cx, cy, rx + 1.0, ry + 1.0, 30, 30, 35);
    fill_ellipse(data, w, h, cx, cy, rx, ry, 140, 140, 150);
}

/// Draw a pointed beak (triangle to the right of face).
fn draw_beak(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, len: i32) {
    for i in 0..len {
        set_pixel(data, w, h, cx as i32 + i, cy as i32, 220, 180, 50, 255);
        if i > 0 {
            set_pixel(data, w, h, cx as i32 + i, cy as i32 - 1, 220, 180, 50, 255);
        }
    }
}

/// Draw spread wings (mirrored pixel lines extending from body).
fn draw_wings(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, span: i32) {
    for i in 0..span {
        let wing_y = cy as i32 - 1 + i / 3;
        set_pixel(data, w, h, cx as i32 - 3 - i, wing_y, 130, 130, 140, 255);
        set_pixel(data, w, h, cx as i32 - 3 - i, wing_y + 1, 130, 130, 140, 255);
        set_pixel(data, w, h, cx as i32 + 3 + i, wing_y, 130, 130, 140, 255);
        set_pixel(data, w, h, cx as i32 + 3 + i, wing_y + 1, 130, 130, 140, 255);
    }
}

/// Small beady bird eyes.
fn draw_bird_eyes(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spacing: f32) {
    fill_circle(data, w, h, cx - spacing, cy, 1.0, 240, 200, 50);
    fill_circle(data, w, h, cx + spacing, cy, 1.0, 240, 200, 50);
}

fn draw_murder_scrounger(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 0.5, 4.5, 4.0);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.0, cy, 3);
    // Burlap satchel (small brown square)
    for py in (cy as i32 + 2)..(cy as i32 + 5) {
        for px in (cx as i32 - 3)..(cx as i32 - 1) {
            set_pixel(data, w, h, px, py, 140, 110, 70, 255);
        }
    }
}

fn draw_sentinel(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 1.0, 4.0, 5.0);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.0, cy, 3);
    // Lookout post (vertical line below)
    for py in (cy as i32 + 4)..(cy as i32 + 8) {
        set_pixel(data, w, h, cx as i32, py, 100, 80, 60, 255);
    }
    // Crossbar
    for px in (cx as i32 - 2)..(cx as i32 + 3) {
        set_pixel(data, w, h, px, cy as i32 + 4, 100, 80, 60, 255);
    }
}

fn draw_rookclaw(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Diving pose (wider than tall)
    draw_bird_body(data, w, h, cx, cy, 5.5, 4.0);
    draw_bird_eyes(data, w, h, cx + 1.0, cy - 1.0, 1.2);
    draw_beak(data, w, h, cx + 4.0, cy - 0.5, 4);
    draw_wings(data, w, h, cx, cy, 4);
    // Extended talons
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 + 2 + i, cy as i32 + 4, 60, 60, 60, 255);
        set_pixel(data, w, h, cx as i32 + 3 + i, cy as i32 + 5, 60, 60, 60, 255);
    }
}

fn draw_magpike(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 0.5, 4.5, 4.0);
    // Iridescent wing shimmer (lighter patches)
    fill_circle(data, w, h, cx - 2.0, cy - 1.0, 1.5, 160, 180, 200);
    fill_circle(data, w, h, cx + 2.0, cy - 1.0, 1.5, 160, 180, 200);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.0, cy, 3);
    // Trinket in beak (shiny dot)
    set_pixel(data, w, h, cx as i32 + 5, cy as i32 - 1, 255, 220, 100, 255);
}

fn draw_magpyre(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 0.5, 4.5, 4.0);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.0, cy, 2);
    // Crossed wires around body
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 3 + i, cy as i32 - 2 + i, 200, 60, 60, 200);
        set_pixel(data, w, h, cx as i32 + 3 - i, cy as i32 - 2 + i, 200, 60, 60, 200);
    }
}

fn draw_jaycaller(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 0.5, 5.0, 5.0);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.5, cy, 3);
    // Puffed chest (lighter center)
    fill_circle(data, w, h, cx, cy + 1.5, 2.5, 170, 170, 180);
    // Spread wings
    draw_wings(data, w, h, cx, cy, 3);
    // Sound waves
    for i in 0..2 {
        set_pixel(data, w, h, cx as i32 + 6, cy as i32 - 1 + i * 2, 200, 200, 220, 180);
    }
}

fn draw_jayflicker(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_bird_body(data, w, h, cx, cy + 0.5, 5.0, 5.0);
    draw_bird_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_beak(data, w, h, cx + 3.5, cy, 2);
    // Afterimage copies (translucent circles flanking)
    fill_circle(data, w, h, cx - 5.0, cy, 2.5, 130, 130, 140);
    fill_circle(data, w, h, cx + 5.0, cy, 2.5, 130, 130, 140);
}

fn draw_dusktalon(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Dark owl on ground, hunched
    fill_ellipse(data, w, h, cx, cy + 1.0, 5.5, 5.0, 25, 25, 30);
    fill_ellipse(data, w, h, cx, cy + 1.0, 4.5, 4.0, 90, 85, 95);
    // Glowing amber eyes
    fill_circle(data, w, h, cx - 2.0, cy - 0.5, 1.5, 255, 180, 40);
    fill_circle(data, w, h, cx + 2.0, cy - 0.5, 1.5, 255, 180, 40);
    fill_circle(data, w, h, cx - 2.0, cy - 0.5, 0.7, 30, 20, 10);
    fill_circle(data, w, h, cx + 2.0, cy - 0.5, 0.7, 30, 20, 10);
}

fn draw_hootseer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large owl
    fill_ellipse(data, w, h, cx, cy, 6.5, 6.0, 30, 30, 35);
    fill_ellipse(data, w, h, cx, cy, 5.5, 5.0, 150, 145, 155);
    // Concentric eye rings
    fill_circle(data, w, h, cx - 2.5, cy - 1.0, 2.5, 220, 200, 100);
    fill_circle(data, w, h, cx + 2.5, cy - 1.0, 2.5, 220, 200, 100);
    fill_circle(data, w, h, cx - 2.5, cy - 1.0, 1.5, 240, 220, 140);
    fill_circle(data, w, h, cx + 2.5, cy - 1.0, 1.5, 240, 220, 140);
    fill_circle(data, w, h, cx - 2.5, cy - 1.0, 0.8, 20, 20, 20);
    fill_circle(data, w, h, cx + 2.5, cy - 1.0, 0.8, 20, 20, 20);
    // Small beak
    draw_beak(data, w, h, cx + 0.5, cy + 1.0, 2);
}

fn draw_corvus_rex(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Massive armored crow — blocky body
    for py in 4..h - 2 {
        for px in 4..w - 4 {
            set_pixel(data, w, h, px as i32, py as i32, 50, 50, 60, 255);
        }
    }
    // Inner body
    for py in 6..h - 4 {
        for px in 6..w - 6 {
            set_pixel(data, w, h, px as i32, py as i32, 120, 120, 135, 255);
        }
    }
    // Glowing eye visor
    for px in (cx as i32 - 4)..(cx as i32 + 4) {
        set_pixel(data, w, h, px, 7, 200, 50, 50, 255);
        set_pixel(data, w, h, px, 8, 200, 50, 50, 255);
    }
    // Armored beak
    draw_beak(data, w, h, cx + 5.0, 9.0, 4);
    // Talons
    for py in (h as i32 - 3)..h as i32 {
        set_pixel(data, w, h, cx as i32 - 4, py, 60, 60, 60, 255);
        set_pixel(data, w, h, cx as i32 + 4, py, 60, 60, 60, 255);
    }
}

// ===== Seekers (Badger) drawing functions =====
// Badgers are stocky, low-slung with broad bodies. White face stripe is signature.

/// Draw a stocky badger body (wider than tall).
fn draw_badger_body(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, rx: f32, ry: f32) {
    fill_ellipse(data, w, h, cx, cy, rx + 1.0, ry + 1.0, 35, 35, 35);
    fill_ellipse(data, w, h, cx, cy, rx, ry, 165, 160, 155);
}

/// Draw the characteristic badger face stripe (white line down center).
fn draw_badger_snout(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, len: i32) {
    for i in 0..len {
        set_pixel(data, w, h, cx as i32, cy as i32 - len / 2 + i, 230, 230, 225, 255);
    }
}

/// Draw digging claws (three prongs below body).
fn draw_claws(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32) {
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 - 2 + i * 2, cy as i32, 80, 70, 60, 255);
        set_pixel(data, w, h, cx as i32 - 2 + i * 2, cy as i32 + 1, 80, 70, 60, 255);
    }
}

/// Small badger/mole eyes (dark dots).
fn draw_badger_eyes(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spacing: f32) {
    fill_circle(data, w, h, cx - spacing, cy, 1.0, 15, 15, 15);
    fill_circle(data, w, h, cx + spacing, cy, 1.0, 15, 15, 15);
}

fn draw_delver(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Squat mole with oversized claws
    draw_badger_body(data, w, h, cx, cy + 0.5, 5.0, 4.5);
    draw_badger_eyes(data, w, h, cx, cy - 0.5, 1.5);
    draw_badger_snout(data, w, h, cx, cy, 3);
    // Mining helmet
    for px in (cx as i32 - 3)..(cx as i32 + 4) {
        set_pixel(data, w, h, px, cy as i32 - 4, 220, 200, 60, 255);
        set_pixel(data, w, h, px, cy as i32 - 3, 220, 200, 60, 255);
    }
    // Big digging claws
    draw_claws(data, w, h, cx - 3.0, cy + 3.0);
    draw_claws(data, w, h, cx + 3.0, cy + 3.0);
}

fn draw_ironhide(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Broad badger with heavy shield
    draw_badger_body(data, w, h, cx, cy, 7.0, 6.5);
    draw_badger_snout(data, w, h, cx, cy - 1.0, 4);
    draw_badger_eyes(data, w, h, cx, cy - 2.0, 2.0);
    // Shield (rectangle to the left)
    for py in (cy as i32 - 4)..(cy as i32 + 4) {
        for px in (cx as i32 - 8)..(cx as i32 - 5) {
            set_pixel(data, w, h, px, py, 100, 100, 110, 255);
        }
    }
}

fn draw_cragback(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Massive badger with boulder mortar on back
    draw_badger_body(data, w, h, cx, cy + 1.0, 8.0, 7.0);
    draw_badger_snout(data, w, h, cx, cy, 4);
    draw_badger_eyes(data, w, h, cx, cy - 1.0, 2.5);
    // Boulder mortar on back (dark circle)
    fill_circle(data, w, h, cx, cy - 5.0, 3.0, 90, 80, 70);
    fill_circle(data, w, h, cx, cy - 5.0, 2.0, 120, 110, 100);
}

fn draw_warden(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_badger_body(data, w, h, cx, cy, 6.0, 5.5);
    draw_badger_snout(data, w, h, cx, cy - 0.5, 4);
    draw_badger_eyes(data, w, h, cx, cy - 1.5, 2.0);
    // Armor plates
    for px in (cx as i32 - 3)..(cx as i32 + 4) {
        set_pixel(data, w, h, px, cy as i32 + 2, 120, 120, 130, 255);
        set_pixel(data, w, h, px, cy as i32 + 3, 120, 120, 130, 255);
    }
}

fn draw_sapjaw(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_badger_body(data, w, h, cx, cy, 6.0, 5.5);
    draw_badger_eyes(data, w, h, cx, cy - 1.0, 2.0);
    // Oversized jaw (wider lower body)
    fill_ellipse(data, w, h, cx, cy + 2.0, 4.0, 2.5, 175, 170, 165);
    // Prominent teeth
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 + 4, 240, 240, 220, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 + 4, 240, 240, 220, 255);
}

fn draw_wardenmother(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Bulky exosuit
    for py in 3..h as i32 - 2 {
        for px in 3..w as i32 - 3 {
            set_pixel(data, w, h, px, py, 70, 70, 80, 255);
        }
    }
    for py in 5..h as i32 - 4 {
        for px in 5..w as i32 - 5 {
            set_pixel(data, w, h, px, py, 150, 145, 140, 255);
        }
    }
    draw_badger_eyes(data, w, h, cx, 7.0, 2.0);
    draw_badger_snout(data, w, h, cx, 8.0, 3);
    // Glowing chest core
    fill_circle(data, w, h, cx, cy + 1.0, 2.0, 100, 220, 100);
}

fn draw_seeker_tunneler(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Streamlined mole mid-burrow
    fill_ellipse(data, w, h, cx, cy, 6.0, 4.0, 35, 35, 35);
    fill_ellipse(data, w, h, cx, cy, 5.0, 3.0, 160, 150, 140);
    draw_badger_eyes(data, w, h, cx + 1.0, cy - 0.5, 1.2);
    // Dirt spray behind
    set_pixel(data, w, h, cx as i32 - 5, cy as i32 - 1, 140, 120, 80, 200);
    set_pixel(data, w, h, cx as i32 - 6, cy as i32, 140, 120, 80, 180);
    set_pixel(data, w, h, cx as i32 - 5, cy as i32 + 1, 140, 120, 80, 160);
}

fn draw_embermaw(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Small fierce wolverine
    draw_badger_body(data, w, h, cx, cy, 6.0, 5.5);
    draw_badger_eyes(data, w, h, cx, cy - 1.0, 2.0);
    draw_badger_snout(data, w, h, cx, cy, 3);
    // Incendiary launcher (tube on shoulder)
    for py in (cy as i32 - 6)..(cy as i32 - 2) {
        set_pixel(data, w, h, cx as i32 + 4, py, 80, 60, 50, 255);
        set_pixel(data, w, h, cx as i32 + 5, py, 80, 60, 50, 255);
    }
    // Flame tip
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 - 7, 255, 160, 40, 255);
    set_pixel(data, w, h, cx as i32 + 5, cy as i32 - 7, 255, 100, 30, 255);
}

fn draw_dustclaw(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Quick mole emerging from dust cloud
    draw_badger_body(data, w, h, cx, cy, 4.5, 4.0);
    draw_badger_eyes(data, w, h, cx, cy - 0.5, 1.5);
    // Dust cloud puffs
    fill_circle(data, w, h, cx - 3.0, cy + 3.0, 1.5, 180, 170, 140);
    fill_circle(data, w, h, cx + 2.0, cy + 4.0, 1.0, 180, 170, 140);
    fill_circle(data, w, h, cx - 1.0, cy + 4.0, 1.2, 180, 170, 140);
}

fn draw_gutripper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Wild wolverine, feral stance
    draw_badger_body(data, w, h, cx, cy, 7.0, 6.0);
    draw_badger_eyes(data, w, h, cx, cy - 1.5, 2.0);
    // Feral eyes (red tint)
    fill_circle(data, w, h, cx - 2.0, cy - 1.5, 1.0, 220, 60, 60);
    fill_circle(data, w, h, cx + 2.0, cy - 1.5, 1.0, 220, 60, 60);
    // Extended claws
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 5 + i, cy as i32 + 5, 80, 70, 60, 255);
        set_pixel(data, w, h, cx as i32 + 2 + i, cy as i32 + 5, 80, 70, 60, 255);
    }
}

// ===== Croak (Axolotl) drawing functions =====
// Axolotls have rounded, bulbous bodies with prominent gills and wide eyes.
// Base color is a lighter, pinkish gray.

/// Draw an axolotl body (round, soft outline).
fn draw_axolotl_body(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, radius: f32) {
    fill_circle(data, w, h, cx, cy, radius + 1.0, 45, 40, 45);
    fill_circle(data, w, h, cx, cy, radius, 195, 180, 190);
}

/// Draw feathery gills (three branching lines on each side of head).
fn draw_gills(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spread: f32) {
    for i in 0..3 {
        let angle_offset = (i as f32 - 1.0) * 0.8;
        // Left gills
        let lx = cx as i32 - (spread + 1.0) as i32;
        let ly = cy as i32 - 2 + i;
        set_pixel(data, w, h, lx, ly, 220, 120, 140, 255);
        set_pixel(data, w, h, lx - 1, ly + angle_offset as i32, 220, 120, 140, 255);
        // Right gills
        let rx = cx as i32 + (spread + 1.0) as i32;
        let ry = cy as i32 - 2 + i;
        set_pixel(data, w, h, rx, ry, 220, 120, 140, 255);
        set_pixel(data, w, h, rx + 1, ry + angle_offset as i32, 220, 120, 140, 255);
    }
}

/// Draw wide, round axolotl eyes.
fn draw_axolotl_eyes(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, spacing: f32) {
    fill_circle(data, w, h, cx - spacing, cy, 1.8, 20, 20, 30);
    fill_circle(data, w, h, cx + spacing, cy, 1.8, 20, 20, 30);
    // Highlight
    fill_circle(data, w, h, cx - spacing - 0.5, cy - 0.5, 0.6, 200, 200, 210);
    fill_circle(data, w, h, cx + spacing - 0.5, cy - 0.5, 0.6, 200, 200, 210);
}

fn draw_ponderer(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_axolotl_body(data, w, h, cx, cy + 0.5, 5.0);
    draw_gills(data, w, h, cx, cy - 1.0, 4.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 0.5, 2.0);
    // Gathering posture (small carried items)
    fill_circle(data, w, h, cx + 3.0, cy + 3.0, 1.0, 100, 180, 200);
}

fn draw_regeneron(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_axolotl_body(data, w, h, cx, cy + 0.5, 4.5);
    draw_gills(data, w, h, cx, cy - 1.0, 3.5);
    draw_axolotl_eyes(data, w, h, cx, cy - 0.5, 1.5);
    // Regenerating limb (glowing stub)
    set_pixel(data, w, h, cx as i32 + 4, cy as i32 + 2, 140, 255, 140, 255);
    set_pixel(data, w, h, cx as i32 + 5, cy as i32 + 2, 100, 220, 100, 200);
}

fn draw_broodmother(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large nurturing axolotl
    draw_axolotl_body(data, w, h, cx, cy, 6.0);
    draw_gills(data, w, h, cx, cy - 2.0, 5.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 1.5, 2.0);
    // Tiny spawn around body
    fill_circle(data, w, h, cx - 4.0, cy + 4.0, 1.0, 195, 180, 190);
    fill_circle(data, w, h, cx + 3.0, cy + 5.0, 1.0, 195, 180, 190);
    fill_circle(data, w, h, cx + 5.0, cy + 3.0, 0.8, 195, 180, 190);
}

fn draw_gulper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Massive-jawed axolotl (wider than tall)
    fill_ellipse(data, w, h, cx, cy, 7.5, 6.0, 45, 40, 45);
    fill_ellipse(data, w, h, cx, cy, 6.5, 5.0, 195, 180, 190);
    draw_gills(data, w, h, cx, cy - 2.0, 5.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 2.0, 2.5);
    // Wide open mouth
    fill_ellipse(data, w, h, cx, cy + 2.5, 4.0, 2.0, 180, 100, 110);
}

fn draw_eftsaber(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Sleek newt (tall, thin)
    fill_ellipse(data, w, h, cx, cy, 4.0, 5.5, 45, 40, 45);
    fill_ellipse(data, w, h, cx, cy, 3.0, 4.5, 180, 170, 175);
    draw_axolotl_eyes(data, w, h, cx, cy - 2.0, 1.2);
    // Twin poison daggers
    for i in 0..3 {
        set_pixel(data, w, h, cx as i32 - 3, cy as i32 + 2 + i, 120, 200, 80, 255);
        set_pixel(data, w, h, cx as i32 + 3, cy as i32 + 2 + i, 120, 200, 80, 255);
    }
}

fn draw_croaker(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Stocky frog with mortar tube
    draw_axolotl_body(data, w, h, cx, cy + 0.5, 5.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 1.5, 2.0);
    // Mortar tube on back
    for py in (cy as i32 - 5)..(cy as i32 - 1) {
        set_pixel(data, w, h, cx as i32 + 2, py, 90, 80, 70, 255);
        set_pixel(data, w, h, cx as i32 + 3, py, 90, 80, 70, 255);
    }
}

fn draw_leapfrog(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Athletic frog mid-leap (wider than tall)
    fill_ellipse(data, w, h, cx, cy, 5.5, 4.0, 45, 40, 45);
    fill_ellipse(data, w, h, cx, cy, 4.5, 3.0, 175, 185, 170);
    draw_axolotl_eyes(data, w, h, cx, cy - 1.5, 1.8);
    // Extended legs
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 4, cy as i32 + 2 + i, 160, 170, 155, 255);
        set_pixel(data, w, h, cx as i32 + 4, cy as i32 + 2 + i, 160, 170, 155, 255);
    }
}

fn draw_shellwarden(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Armored turtle hunched behind shell
    fill_ellipse(data, w, h, cx, cy, 6.5, 5.5, 60, 70, 55);
    fill_ellipse(data, w, h, cx, cy, 5.5, 4.5, 120, 140, 110);
    // Shell pattern (cross lines)
    for i in -3..4 {
        set_pixel(data, w, h, cx as i32 + i, cy as i32, 90, 110, 80, 255);
    }
    for i in -2..3 {
        set_pixel(data, w, h, cx as i32, cy as i32 + i, 90, 110, 80, 255);
    }
    // Head poking out
    fill_circle(data, w, h, cx + 5.0, cy - 1.0, 2.0, 175, 185, 170);
    draw_axolotl_eyes(data, w, h, cx + 5.0, cy - 1.5, 0.8);
}

fn draw_bogwhisper(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_axolotl_body(data, w, h, cx, cy, 5.0);
    draw_gills(data, w, h, cx, cy - 1.5, 4.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 0.5, 2.0);
    // Swirling water tendrils
    for i in 0..4 {
        let tx = cx as i32 - 4 + i * 2;
        set_pixel(data, w, h, tx, cy as i32 + 4 + (i % 2), 80, 160, 220, 200);
        set_pixel(data, w, h, tx, cy as i32 + 5 + (i % 2), 80, 160, 220, 160);
    }
}

fn draw_murk_commander(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large axolotl in tech armor
    fill_ellipse(data, w, h, cx, cy, 9.0, 8.0, 50, 50, 55);
    fill_ellipse(data, w, h, cx, cy, 8.0, 7.0, 160, 155, 165);
    draw_gills(data, w, h, cx, cy - 4.0, 6.0);
    draw_axolotl_eyes(data, w, h, cx, cy - 2.0, 3.0);
    // Command antenna
    set_pixel(data, w, h, cx as i32, cy as i32 - 9, 200, 60, 60, 255);
    set_pixel(data, w, h, cx as i32, cy as i32 - 10, 200, 60, 60, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 - 11, 200, 60, 60, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 - 11, 200, 60, 60, 255);
    // Tech armor plates
    for px in (cx as i32 - 4)..(cx as i32 + 5) {
        set_pixel(data, w, h, px, cy as i32 + 2, 100, 100, 120, 255);
    }
}

// ===== LLAMA (Raccoon) drawing functions =====
// Raccoons have round faces with distinctive eye mask markings and ringed tails.
// Medium-build bodies, scrappy/improvised equipment aesthetic.

/// Draw a raccoon body (round, medium gray).
fn draw_raccoon_body(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, radius: f32) {
    fill_circle(data, w, h, cx, cy, radius + 1.0, 40, 40, 40);
    fill_circle(data, w, h, cx, cy, radius, 175, 170, 165);
}

/// Draw raccoon eye mask (dark band across eyes).
fn draw_mask_markings(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, width: f32) {
    for px in (cx as i32 - width as i32)..(cx as i32 + width as i32 + 1) {
        set_pixel(data, w, h, px, cy as i32, 50, 45, 40, 255);
        set_pixel(data, w, h, px, cy as i32 - 1, 50, 45, 40, 255);
    }
    // Bright eyes within mask
    fill_circle(data, w, h, cx - width * 0.5, cy - 0.5, 1.0, 200, 200, 200);
    fill_circle(data, w, h, cx + width * 0.5, cy - 0.5, 1.0, 200, 200, 200);
    fill_circle(data, w, h, cx - width * 0.5, cy - 0.3, 0.5, 20, 20, 20);
    fill_circle(data, w, h, cx + width * 0.5, cy - 0.3, 0.5, 20, 20, 20);
}

/// Draw a ringed raccoon tail (alternating dark/light bands).
fn draw_ringed_tail(data: &mut [u8], w: usize, h: usize, cx: f32, cy: f32) {
    for i in 0..6 {
        let tx = cx as i32 + 2 + i;
        let ty = cy as i32 + 2 + (i / 2);
        let color = if i % 2 == 0 { (80, 75, 70) } else { (175, 170, 165) };
        set_pixel(data, w, h, tx, ty, color.0, color.1, color.2, 255);
        set_pixel(data, w, h, tx, ty + 1, color.0, color.1, color.2, 255);
    }
}

fn draw_scrounger(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_raccoon_body(data, w, h, cx, cy + 0.5, 4.5);
    draw_mask_markings(data, w, h, cx, cy - 0.5, 3.0);
    draw_ringed_tail(data, w, h, cx, cy + 1.0);
    // Carrying scrap
    for px in (cx as i32 - 2)..(cx as i32) {
        set_pixel(data, w, h, px, cy as i32 + 3, 140, 130, 100, 255);
    }
}

fn draw_bandit(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_raccoon_body(data, w, h, cx, cy + 0.5, 4.5);
    draw_mask_markings(data, w, h, cx, cy - 0.5, 3.0);
    draw_ringed_tail(data, w, h, cx, cy + 1.0);
    // Bandana (red triangle on forehead)
    set_pixel(data, w, h, cx as i32, cy as i32 - 4, 200, 60, 60, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 - 3, 200, 60, 60, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 - 3, 200, 60, 60, 255);
}

fn draw_heap_titan(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Massive raccoon in scrap armor
    for py in 3..h as i32 - 2 {
        for px in 3..w as i32 - 3 {
            set_pixel(data, w, h, px, py, 80, 75, 70, 255);
        }
    }
    for py in 5..h as i32 - 4 {
        for px in 5..w as i32 - 5 {
            set_pixel(data, w, h, px, py, 155, 150, 140, 255);
        }
    }
    draw_mask_markings(data, w, h, cx, 7.0, 3.0);
    // Bolts/rivets on armor
    set_pixel(data, w, h, cx as i32 - 6, cy as i32 - 2, 120, 120, 120, 255);
    set_pixel(data, w, h, cx as i32 + 6, cy as i32 - 2, 120, 120, 120, 255);
    set_pixel(data, w, h, cx as i32 - 6, cy as i32 + 2, 120, 120, 120, 255);
    set_pixel(data, w, h, cx as i32 + 6, cy as i32 + 2, 120, 120, 120, 255);
}

fn draw_glitch_rat(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Wiry rat with sparking wires
    fill_ellipse(data, w, h, cx, cy + 0.5, 4.0, 4.5, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy + 0.5, 3.0, 3.5, 160, 150, 145);
    // Beady eyes
    fill_circle(data, w, h, cx - 1.0, cy - 1.0, 0.8, 20, 20, 20);
    fill_circle(data, w, h, cx + 1.0, cy - 1.0, 0.8, 20, 20, 20);
    // Sparking wires
    set_pixel(data, w, h, cx as i32 + 2, cy as i32 - 3, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 - 2, cy as i32 - 2, 255, 255, 100, 255);
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 + 1, 255, 255, 100, 255);
}

fn draw_patch_possum(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Possum with duct tape bandolier
    fill_ellipse(data, w, h, cx, cy + 0.5, 4.5, 5.0, 40, 40, 40);
    fill_ellipse(data, w, h, cx, cy + 0.5, 3.5, 4.0, 185, 180, 175);
    // Beady eyes
    fill_circle(data, w, h, cx - 1.5, cy - 1.0, 0.8, 20, 20, 20);
    fill_circle(data, w, h, cx + 1.5, cy - 1.0, 0.8, 20, 20, 20);
    // Medic cross patch
    set_pixel(data, w, h, cx as i32, cy as i32 + 1, 220, 50, 50, 255);
    set_pixel(data, w, h, cx as i32 - 1, cy as i32 + 2, 220, 50, 50, 255);
    set_pixel(data, w, h, cx as i32, cy as i32 + 2, 220, 50, 50, 255);
    set_pixel(data, w, h, cx as i32 + 1, cy as i32 + 2, 220, 50, 50, 255);
    set_pixel(data, w, h, cx as i32, cy as i32 + 3, 220, 50, 50, 255);
    // Duct tape bandolier (diagonal stripe)
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 2 + i, cy as i32 - 3 + i, 180, 180, 180, 200);
    }
}

fn draw_grease_monkey(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_raccoon_body(data, w, h, cx, cy + 0.5, 5.0);
    draw_mask_markings(data, w, h, cx, cy - 0.5, 3.0);
    draw_ringed_tail(data, w, h, cx, cy + 2.0);
    // Goggles (circles around eyes, over mask)
    fill_circle(data, w, h, cx - 1.5, cy - 1.0, 2.0, 100, 80, 50);
    fill_circle(data, w, h, cx + 1.5, cy - 1.0, 2.0, 100, 80, 50);
    fill_circle(data, w, h, cx - 1.5, cy - 1.0, 1.2, 160, 200, 220);
    fill_circle(data, w, h, cx + 1.5, cy - 1.0, 1.2, 160, 200, 220);
    // Junk launcher on shoulder
    for py in (cy as i32 - 5)..(cy as i32 - 2) {
        set_pixel(data, w, h, cx as i32 + 4, py, 100, 90, 80, 255);
    }
}

fn draw_dead_drop_unit(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Hooded raccoon in shadow
    fill_ellipse(data, w, h, cx, cy + 0.5, 4.5, 5.0, 30, 30, 35);
    fill_ellipse(data, w, h, cx, cy + 0.5, 3.5, 4.0, 100, 95, 95);
    // Hood (dark arc on top)
    fill_circle(data, w, h, cx, cy - 2.0, 3.0, 50, 45, 45);
    // Glinting eyes in shadow
    fill_circle(data, w, h, cx - 1.2, cy - 0.5, 0.8, 180, 200, 180);
    fill_circle(data, w, h, cx + 1.2, cy - 0.5, 0.8, 180, 200, 180);
}

fn draw_wrecker(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_raccoon_body(data, w, h, cx, cy, 6.0);
    draw_mask_markings(data, w, h, cx, cy - 1.5, 3.5);
    // Crowbar (diagonal line)
    for i in 0..6 {
        set_pixel(data, w, h, cx as i32 + 3 + i, cy as i32 - 3 + i, 100, 100, 110, 255);
    }
    // Hook at top of crowbar
    set_pixel(data, w, h, cx as i32 + 3, cy as i32 - 4, 100, 100, 110, 255);
    // Demolition harness (X across chest)
    for i in 0..4 {
        set_pixel(data, w, h, cx as i32 - 2 + i, cy as i32 + i, 140, 100, 40, 200);
        set_pixel(data, w, h, cx as i32 + 2 - i, cy as i32 + i, 140, 100, 40, 200);
    }
}

fn draw_dumpster_diver(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    draw_raccoon_body(data, w, h, cx, cy - 0.5, 5.0);
    draw_mask_markings(data, w, h, cx, cy - 1.5, 3.0);
    // Dumpster (rectangle below, raccoon climbing out)
    for py in (cy as i32 + 2)..(cy as i32 + 6) {
        for px in (cx as i32 - 5)..(cx as i32 + 6) {
            set_pixel(data, w, h, px, py, 80, 100, 80, 255);
        }
    }
    // Treasure (shiny dot in paw)
    set_pixel(data, w, h, cx as i32 + 3, cy as i32, 255, 220, 80, 255);
}

fn draw_junkyard_king(data: &mut [u8], w: usize, h: usize) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Large raccoon on throne of scrap
    draw_raccoon_body(data, w, h, cx, cy - 1.0, 7.0);
    draw_mask_markings(data, w, h, cx, cy - 2.5, 4.0);
    // Crown of bent forks
    for i in 0..5 {
        let fx = cx as i32 - 4 + i * 2;
        set_pixel(data, w, h, fx, cy as i32 - 8, 200, 190, 140, 255);
        set_pixel(data, w, h, fx, cy as i32 - 9, 200, 190, 140, 255);
    }
    // Scrap throne (blocky base)
    for py in (cy as i32 + 4)..(cy as i32 + 8) {
        for px in (cx as i32 - 8)..(cx as i32 + 9) {
            set_pixel(data, w, h, px, py, 110, 100, 90, 255);
        }
    }
    draw_ringed_tail(data, w, h, cx + 2.0, cy + 1.0);
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
        // Murder (corvid) paths
        assert_eq!(sprite_file_path(UnitKind::MurderScrounger), "sprites/units/murder_scrounger_idle.png");
        assert_eq!(sprite_file_path(UnitKind::CorvusRex), "sprites/units/corvus_rex_idle.png");
        // Seekers (badger) paths
        assert_eq!(sprite_file_path(UnitKind::Delver), "sprites/units/delver_idle.png");
        assert_eq!(sprite_file_path(UnitKind::Gutripper), "sprites/units/gutripper_idle.png");
        // Croak (axolotl) paths
        assert_eq!(sprite_file_path(UnitKind::Ponderer), "sprites/units/ponderer_idle.png");
        assert_eq!(sprite_file_path(UnitKind::MurkCommander), "sprites/units/murk_commander_idle.png");
        // LLAMA (raccoon) paths
        assert_eq!(sprite_file_path(UnitKind::Scrounger), "sprites/units/scrounger_idle.png");
        assert_eq!(sprite_file_path(UnitKind::JunkyardKing), "sprites/units/junkyard_king_idle.png");
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
    fn all_kinds_constant_has_sixty_entries() {
        assert_eq!(ALL_KINDS.len(), 60);
    }

    /// Cat units (0-9) should have art files on disk.
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
    fn non_cat_unit_slugs_are_valid() {
        // Check all 50 non-cat units (indices 10-59) have valid, unique slugs
        for kind in &ALL_KINDS[10..] {
            let slug = unit_slug(*kind);
            assert!(!slug.is_empty(), "Empty slug for {kind:?}");
            assert_ne!(slug, "pawdler", "Unit {kind:?} should not fall through to pawdler");
        }
    }

    #[test]
    fn all_slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for kind in ALL_KINDS {
            let slug = unit_slug(kind);
            assert!(seen.insert(slug), "Duplicate slug: {slug} for {kind:?}");
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
