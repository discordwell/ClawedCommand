use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::components::ProjectileKind;

/// Resource holding per-kind projectile sprite handles.
/// Falls back to procedurally generated colored sprites if no art exists on disk.
#[derive(Resource)]
pub struct ProjectileSprites {
    pub sprites: [Handle<Image>; 6],
}

/// Map ProjectileKind to array index.
pub fn kind_to_index(kind: ProjectileKind) -> usize {
    match kind {
        ProjectileKind::Spit => 0,
        ProjectileKind::LaserBeam => 1,
        ProjectileKind::SonicWave => 2,
        ProjectileKind::MechShot => 3,
        ProjectileKind::Explosive => 4,
        ProjectileKind::Generic => 5,
    }
}

/// Fallback color per projectile kind.
pub fn kind_color(kind: ProjectileKind) -> Color {
    match kind {
        ProjectileKind::Spit => Color::srgb(0.3, 0.9, 0.2),       // Green
        ProjectileKind::LaserBeam => Color::srgb(1.0, 0.2, 0.2),   // Red
        ProjectileKind::SonicWave => Color::srgb(0.7, 0.3, 1.0),   // Purple
        ProjectileKind::MechShot => Color::srgb(0.3, 0.9, 1.0),    // Cyan
        ProjectileKind::Explosive => Color::srgb(1.0, 0.6, 0.1),   // Orange
        ProjectileKind::Generic => Color::srgb(1.0, 0.9, 0.3),     // Yellow
    }
}

/// Projectile size per kind.
pub fn kind_size(kind: ProjectileKind) -> Vec2 {
    match kind {
        ProjectileKind::Spit => Vec2::new(5.0, 5.0),
        ProjectileKind::LaserBeam => Vec2::new(8.0, 3.0),
        ProjectileKind::SonicWave => Vec2::new(7.0, 7.0),
        ProjectileKind::MechShot => Vec2::new(6.0, 4.0),
        ProjectileKind::Explosive => Vec2::new(6.0, 6.0),
        ProjectileKind::Generic => Vec2::new(4.0, 4.0),
    }
}

const ALL_KINDS: [ProjectileKind; 6] = [
    ProjectileKind::Spit,
    ProjectileKind::LaserBeam,
    ProjectileKind::SonicWave,
    ProjectileKind::MechShot,
    ProjectileKind::Explosive,
    ProjectileKind::Generic,
];

fn kind_slug(kind: ProjectileKind) -> &'static str {
    match kind {
        ProjectileKind::Spit => "spit",
        ProjectileKind::LaserBeam => "laser_beam",
        ProjectileKind::SonicWave => "sonic_wave",
        ProjectileKind::MechShot => "mech_shot",
        ProjectileKind::Explosive => "explosive",
        ProjectileKind::Generic => "generic",
    }
}

/// Generate a small procedural projectile sprite (8x8 pixel).
fn generate_projectile_image(kind: ProjectileKind) -> Image {
    let size = 8u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let (r, g, b) = match kind {
        ProjectileKind::Spit => (76, 230, 51),
        ProjectileKind::LaserBeam => (255, 51, 51),
        ProjectileKind::SonicWave => (179, 76, 255),
        ProjectileKind::MechShot => (76, 230, 255),
        ProjectileKind::Explosive => (255, 153, 25),
        ProjectileKind::Generic => (255, 230, 76),
    };

    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let radius = 3.0f32;
    let r2 = radius * radius;

    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 - cx + 0.5;
            let dy = py as f32 - cy + 0.5;
            if dx * dx + dy * dy <= r2 {
                let idx = ((py * size + px) * 4) as usize;
                // Brighter center, dimmer edges
                let dist_frac = (dx * dx + dy * dy).sqrt() / radius;
                let brightness = (1.0 - dist_frac * 0.4).max(0.6);
                data[idx] = (r as f32 * brightness).min(255.0) as u8;
                data[idx + 1] = (g as f32 * brightness).min(255.0) as u8;
                data[idx + 2] = (b as f32 * brightness).min(255.0) as u8;
                data[idx + 3] = 255;
            }
        }
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// Load projectile sprites at startup. Tries disk first, falls back to procedural.
pub fn load_projectile_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(6);

    for kind in ALL_KINDS {
        let slug = kind_slug(kind);
        let asset_path = format!("sprites/projectiles/{slug}.png");

        #[cfg(not(target_arch = "wasm32"))]
        let use_disk = std::path::Path::new("assets").join(&asset_path).exists();
        #[cfg(target_arch = "wasm32")]
        let use_disk = false;

        if use_disk {
            handles.push(asset_server.load(asset_path));
        } else {
            let img = generate_projectile_image(kind);
            handles.push(images.add(img));
        }
    }

    commands.insert_resource(ProjectileSprites {
        sprites: handles.try_into().expect("exactly 6 projectile sprites"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_colors() {
        for kind in ALL_KINDS {
            let _color = kind_color(kind);
        }
    }

    #[test]
    fn all_kinds_have_sizes() {
        for kind in ALL_KINDS {
            let size = kind_size(kind);
            assert!(size.x > 0.0);
            assert!(size.y > 0.0);
        }
    }

    #[test]
    fn all_kinds_have_slugs() {
        for kind in ALL_KINDS {
            let slug = kind_slug(kind);
            assert!(!slug.is_empty());
        }
    }

    #[test]
    fn kind_to_index_covers_all() {
        for (i, kind) in ALL_KINDS.iter().enumerate() {
            assert_eq!(kind_to_index(*kind), i);
        }
    }

    #[test]
    fn procedural_images_are_nonzero() {
        for kind in ALL_KINDS {
            let img = generate_projectile_image(kind);
            let data = img.data.as_ref().expect("Image should have data");
            let has_nonzero = data.iter().any(|&b| b != 0);
            assert!(has_nonzero, "Procedural image for {kind:?} is all zeros");
        }
    }
}
