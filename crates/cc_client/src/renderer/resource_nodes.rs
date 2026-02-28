use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::map_format::ResourceKind;

/// Resource holding procedurally generated resource node icons.
#[derive(Resource)]
pub struct ResourceNodeSprites {
    pub fish_pond: Handle<Image>,
    pub berry_bush: Handle<Image>,
    pub gpu_deposit: Handle<Image>,
    pub monkey_mine: Handle<Image>,
}

impl ResourceNodeSprites {
    pub fn get(&self, kind: ResourceKind) -> Handle<Image> {
        match kind {
            ResourceKind::FishPond => self.fish_pond.clone(),
            ResourceKind::BerryBush => self.berry_bush.clone(),
            ResourceKind::GpuDeposit => self.gpu_deposit.clone(),
            ResourceKind::MonkeyMine => self.monkey_mine.clone(),
        }
    }
}

/// Generate resource node sprite images at startup.
pub fn generate_resource_sprites(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(ResourceNodeSprites {
        fish_pond: images.add(generate_fish_pond()),
        berry_bush: images.add(generate_berry_bush()),
        gpu_deposit: images.add(generate_gpu_deposit()),
        monkey_mine: images.add(generate_monkey_mine()),
    });
}

const SIZE: usize = 20;

fn new_image(data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

fn set_px(data: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x >= 0 && y >= 0 && (x as usize) < SIZE && (y as usize) < SIZE {
        let idx = (y as usize * SIZE + x as usize) * 4;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
        data[idx + 3] = a;
    }
}

fn fill_circle(data: &mut [u8], cx: f32, cy: f32, radius: f32, r: u8, g: u8, b: u8, a: u8) {
    let r2 = radius * radius;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                set_px(data, px as i32, py as i32, r, g, b, a);
            }
        }
    }
}

/// Blue circle with white ripple arcs.
fn generate_fish_pond() -> Image {
    let mut data = vec![0u8; SIZE * SIZE * 4];
    let c = SIZE as f32 / 2.0;
    // Water circle
    fill_circle(&mut data, c, c, 8.0, 60, 140, 220, 255);
    // Ripple arcs (white partial circles)
    for angle in 0..12 {
        let a = angle as f32 * 0.5;
        let rx = c + (a.cos() * 4.0);
        let ry = c + (a.sin() * 3.0);
        set_px(&mut data, rx as i32, ry as i32, 200, 220, 255, 200);
    }
    // Outer ripple
    for angle in 0..16 {
        let a = angle as f32 * 0.4;
        let rx = c + (a.cos() * 6.0);
        let ry = c + (a.sin() * 5.0);
        set_px(&mut data, rx as i32, ry as i32, 180, 200, 240, 150);
    }
    new_image(data)
}

/// Green blob with red/purple dots.
fn generate_berry_bush() -> Image {
    let mut data = vec![0u8; SIZE * SIZE * 4];
    let c = SIZE as f32 / 2.0;
    // Green bush body
    fill_circle(&mut data, c, c, 7.0, 60, 140, 50, 255);
    fill_circle(&mut data, c - 2.0, c - 1.0, 5.0, 70, 160, 60, 255);
    fill_circle(&mut data, c + 2.0, c + 1.0, 5.0, 65, 150, 55, 255);
    // Berry dots
    let berries = [(c - 3.0, c - 2.0), (c + 2.0, c - 1.0), (c, c + 3.0), (c - 1.0, c + 1.0), (c + 3.0, c + 2.0)];
    for (bx, by) in berries {
        fill_circle(&mut data, bx, by, 1.5, 200, 50, 80, 255);
    }
    new_image(data)
}

/// Gray rectangle with green LED dots.
fn generate_gpu_deposit() -> Image {
    let mut data = vec![0u8; SIZE * SIZE * 4];
    // Gray rectangle body
    for py in 4..16 {
        for px in 3..17 {
            set_px(&mut data, px, py, 100, 100, 110, 255);
        }
    }
    // Border
    for py in 3..17 {
        set_px(&mut data, 2, py, 70, 70, 80, 255);
        set_px(&mut data, 17, py, 70, 70, 80, 255);
    }
    for px in 2..18 {
        set_px(&mut data, px, 3, 70, 70, 80, 255);
        set_px(&mut data, px, 16, 70, 70, 80, 255);
    }
    // Green LED dots in grid pattern
    for row in 0..3 {
        for col in 0..3 {
            let x = 6 + col * 4;
            let y = 6 + row * 3;
            set_px(&mut data, x, y, 50, 230, 50, 255);
        }
    }
    new_image(data)
}

/// Gold diamond outline.
fn generate_monkey_mine() -> Image {
    let mut data = vec![0u8; SIZE * SIZE * 4];
    let c = SIZE as f32 / 2.0;
    // Diamond outline
    for py in 0..SIZE {
        for px in 0..SIZE {
            let dx = (px as f32 - c).abs() / 7.0;
            let dy = (py as f32 - c).abs() / 7.0;
            let d = dx + dy;
            if d <= 1.0 && d >= 0.75 {
                set_px(&mut data, px as i32, py as i32, 220, 180, 30, 255);
            } else if d < 0.75 {
                set_px(&mut data, px as i32, py as i32, 180, 140, 20, 200);
            }
        }
    }
    // NFT text indicator (small 'N' in center)
    set_px(&mut data, 8, 8, 255, 220, 50, 255);
    set_px(&mut data, 8, 9, 255, 220, 50, 255);
    set_px(&mut data, 8, 10, 255, 220, 50, 255);
    set_px(&mut data, 8, 11, 255, 220, 50, 255);
    set_px(&mut data, 9, 9, 255, 220, 50, 255);
    set_px(&mut data, 10, 10, 255, 220, 50, 255);
    set_px(&mut data, 11, 8, 255, 220, 50, 255);
    set_px(&mut data, 11, 9, 255, 220, 50, 255);
    set_px(&mut data, 11, 10, 255, 220, 50, 255);
    set_px(&mut data, 11, 11, 255, 220, 50, 255);
    new_image(data)
}
