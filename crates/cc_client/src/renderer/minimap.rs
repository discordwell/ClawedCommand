use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::setup::UnitMesh;
use cc_core::components::{Owner, Position};
use cc_core::coords;
use cc_core::terrain::TerrainType;
use cc_sim::resources::MapResource;

/// Minimap layout constants — shared between setup and click detection.
pub const MINIMAP_LEFT: f32 = 10.0;
pub const MINIMAP_BOTTOM: f32 = 10.0;
pub const MINIMAP_SIZE: f32 = 200.0;
pub const MINIMAP_BORDER: f32 = 2.0;

/// Timer-gated minimap refresh (every 0.3s).
#[derive(Resource)]
pub struct MinimapTimer(pub Timer);

/// Handle to the minimap image.
#[derive(Resource)]
pub struct MinimapImage(pub Handle<Image>);

/// Marker for the minimap UI node.
#[derive(Component)]
pub struct MinimapNode;

/// Flag set when minimap consumes a click, checked by handle_mouse_input to skip.
#[derive(Resource, Default)]
pub struct MinimapClickConsumed(pub bool);

/// Write a single pixel into the RGBA data buffer.
#[inline]
fn write_pixel(data: &mut [u8], idx: usize, r: u8, g: u8, b: u8) {
    let base = idx * 4;
    data[base] = r;
    data[base + 1] = g;
    data[base + 2] = b;
    data[base + 3] = 255;
}

/// Paint terrain colors into the minimap data buffer.
fn paint_terrain(data: &mut [u8], map: &cc_core::map::GameMap) {
    let w = map.width as usize;
    let h = map.height as usize;
    for y in 0..h {
        for x in 0..w {
            let grid = cc_core::coords::GridPos::new(x as i32, y as i32);
            if let Some(tile) = map.get(grid) {
                let (r, g, b) = minimap_terrain_color(tile.terrain);
                write_pixel(data, y * w + x, r, g, b);
            }
        }
    }
}

/// Initialize the minimap image and UI node.
pub fn setup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    map_res: Res<MapResource>,
) {
    let w = map_res.map.width;
    let h = map_res.map.height;

    // Create RGBA image with correct size
    let size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    // Paint initial terrain so it's not black on first frame
    if let Some(data) = image.data.as_mut() {
        paint_terrain(data, &map_res.map);
    }

    let image_handle = images.add(image);

    commands.insert_resource(MinimapImage(image_handle.clone()));
    commands.insert_resource(MinimapTimer(Timer::from_seconds(0.3, TimerMode::Repeating)));

    // UI node: bottom-left corner, using shared constants
    commands
        .spawn((
            MinimapNode,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MINIMAP_LEFT),
                bottom: Val::Px(MINIMAP_BOTTOM),
                width: Val::Px(MINIMAP_SIZE),
                height: Val::Px(MINIMAP_SIZE),
                border: UiRect::all(Val::Px(MINIMAP_BORDER)),
                ..default()
            },
            ZIndex(100),
            BorderColor::all(Color::srgba(0.8, 0.8, 0.8, 0.6)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(image_handle),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
        });
}

/// Update the minimap image with terrain colors, unit dots, and viewport indicator.
pub fn update_minimap(
    time: Res<Time>,
    mut timer: ResMut<MinimapTimer>,
    minimap_img: Option<Res<MinimapImage>>,
    mut images: ResMut<Assets<Image>>,
    map_res: Res<MapResource>,
    units: Query<(&Position, &Owner), With<UnitMesh>>,
    camera_q: Single<(&Transform, &Projection), With<Camera2d>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Some(minimap_img) = minimap_img else {
        return;
    };

    let Some(image) = images.get_mut(&minimap_img.0) else {
        return;
    };

    let map = &map_res.map;
    let w = map.width as usize;
    let h = map.height as usize;

    let Some(data) = image.data.as_mut() else {
        return;
    };

    paint_terrain(data, map);

    // Write unit dots
    for (pos, owner) in units.iter() {
        let grid = pos.world.to_grid();
        if grid.x >= 0 && grid.y >= 0 && (grid.x as u32) < map.width && (grid.y as u32) < map.height
        {
            let idx = grid.y as usize * w + grid.x as usize;
            let (r, g, b) = if owner.player_id == 0 {
                (50, 100, 230) // Blue
            } else {
                (230, 50, 50) // Red
            };
            write_pixel(data, idx, r, g, b);
        }
    }

    // Draw viewport indicator (white rectangle outline)
    let (cam_transform, projection) = *camera_q;
    let Projection::Orthographic(ref ortho) = *projection else {
        return;
    };

    let cam_x = cam_transform.translation.x;
    let cam_y = cam_transform.translation.y;

    // Compute visible screen-space bounds from camera (accounting for zoom scale)
    let half_w = (ortho.area.max.x - ortho.area.min.x) * 0.5 * ortho.scale;
    let half_h = (ortho.area.max.y - ortho.area.min.y) * 0.5 * ortho.scale;

    // 4 viewport corners in bevy world space → isometric world → grid
    let corners = [
        (cam_x - half_w, cam_y - half_h),
        (cam_x + half_w, cam_y - half_h),
        (cam_x - half_w, cam_y + half_h),
        (cam_x + half_w, cam_y + half_h),
    ];

    let mut min_gx = i32::MAX;
    let mut max_gx = i32::MIN;
    let mut min_gy = i32::MAX;
    let mut max_gy = i32::MIN;

    for &(sx, sy) in &corners {
        // Bevy Y-up → iso Y-down: flip Y
        let iso = coords::screen_to_world(coords::ScreenPos { x: sx, y: -sy });
        let grid = iso.to_grid();
        min_gx = min_gx.min(grid.x);
        max_gx = max_gx.max(grid.x);
        min_gy = min_gy.min(grid.y);
        max_gy = max_gy.max(grid.y);
    }

    // Clamp to map bounds; guard against negative values before usize cast
    let min_gx = min_gx.max(0) as usize;
    let max_gx = (max_gx.max(0) as usize).min(w.saturating_sub(1));
    let min_gy = min_gy.max(0) as usize;
    let max_gy = (max_gy.max(0) as usize).min(h.saturating_sub(1));

    // Skip if viewport is completely off-map
    if min_gx > max_gx || min_gy > max_gy {
        return;
    }

    // Draw white outline
    for x in min_gx..=max_gx {
        write_pixel(data, min_gy * w + x, 255, 255, 255); // Top edge
        write_pixel(data, max_gy * w + x, 255, 255, 255); // Bottom edge
    }
    for y in min_gy..=max_gy {
        write_pixel(data, y * w + min_gx, 255, 255, 255); // Left edge
        write_pixel(data, y * w + max_gx, 255, 255, 255); // Right edge
    }
}

/// Minimap click system — pans camera when player clicks inside the minimap.
/// Runs before mouse input so it can consume clicks.
pub fn minimap_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    map_res: Res<MapResource>,
    mut camera_q: Single<&mut Transform, With<Camera2d>>,
    mut consumed: ResMut<MinimapClickConsumed>,
) {
    // Track whether the current press started on the minimap.
    // On just_pressed, check if cursor is inside minimap → set consumed.
    // While pressed and consumed, continue panning.
    // On release, reset consumed.
    if mouse_button.just_released(MouseButton::Left) {
        // Don't reset consumed until AFTER mouse input has run this frame.
        // The flag persists for one frame so handle_mouse_input sees it.
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        consumed.0 = false; // Reset on new press
    }

    if !mouse_button.pressed(MouseButton::Left) {
        consumed.0 = false;
        return;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let win_h = window.height();

    // Minimap content area (inside border)
    let content_left = MINIMAP_LEFT + MINIMAP_BORDER;
    let content_bottom = MINIMAP_BOTTOM + MINIMAP_BORDER;
    let content_size = MINIMAP_SIZE - MINIMAP_BORDER * 2.0;

    // Screen-space: origin top-left, Y increases downward
    let minimap_top = win_h - content_bottom - content_size;
    let minimap_left = content_left;

    // Check if cursor is inside minimap content bounds
    let in_minimap = cursor_pos.x >= minimap_left
        && cursor_pos.x <= minimap_left + content_size
        && cursor_pos.y >= minimap_top
        && cursor_pos.y <= minimap_top + content_size;

    if mouse_button.just_pressed(MouseButton::Left) && in_minimap {
        consumed.0 = true;
    }

    if !consumed.0 {
        return;
    }

    // Even if dragged outside, keep panning while consumed
    // But clamp to minimap bounds for coordinate conversion
    let cx = (cursor_pos.x - minimap_left).clamp(0.0, content_size);
    let cy = (cursor_pos.y - minimap_top).clamp(0.0, content_size);

    // Normalized [0, 1]
    let nx = cx / content_size;
    let ny = cy / content_size;

    // Grid coordinates (float)
    let gx = nx * map_res.map.width as f32;
    let gy = ny * map_res.map.height as f32;

    // Convert grid → world → screen → camera translation
    let world_pos = cc_core::coords::WorldPos {
        x: cc_core::math::Fixed::from_num(gx),
        y: cc_core::math::Fixed::from_num(gy),
    };
    let screen_pos = coords::world_to_screen(world_pos);
    camera_q.translation.x = screen_pos.x;
    camera_q.translation.y = -screen_pos.y; // Flip for Bevy's Y-up
}

fn minimap_terrain_color(terrain: TerrainType) -> (u8, u8, u8) {
    match terrain {
        TerrainType::Grass => (72, 140, 64),
        TerrainType::Dirt => (140, 107, 72),
        TerrainType::Sand => (217, 199, 140),
        TerrainType::Forest => (46, 107, 38),
        TerrainType::Water => (38, 89, 166),
        TerrainType::Shallows => (107, 178, 230),
        TerrainType::Rock => (89, 82, 77),
        TerrainType::Ramp => (128, 115, 97),
        TerrainType::Road => (158, 138, 107),
        TerrainType::TechRuins => (110, 110, 122),
        TerrainType::Concrete => (184, 179, 173),
        TerrainType::Linoleum => (199, 189, 166),
        TerrainType::CarpetTile => (115, 122, 140),
        TerrainType::MetalGrate => (97, 102, 107),
        TerrainType::DryWall => (217, 212, 204),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_click_consumed_default_false() {
        let consumed = MinimapClickConsumed::default();
        assert!(!consumed.0);
    }

    #[test]
    fn minimap_terrain_colors_are_distinct() {
        use std::collections::HashSet;
        let variants = TerrainType::ALL;
        let mut colors = HashSet::new();
        for t in &variants {
            colors.insert(minimap_terrain_color(*t));
        }
        assert_eq!(
            colors.len(),
            variants.len(),
            "each terrain should have a unique color"
        );
    }

    #[test]
    fn minimap_constants_consistent() {
        // Content area should be positive
        let content = MINIMAP_SIZE - MINIMAP_BORDER * 2.0;
        assert!(content > 0.0);
        assert!(MINIMAP_LEFT >= 0.0);
        assert!(MINIMAP_BOTTOM >= 0.0);
    }

    #[test]
    fn write_pixel_sets_rgba() {
        let mut data = vec![0u8; 16]; // 4 pixels
        write_pixel(&mut data, 2, 128, 64, 32);
        assert_eq!(data[8], 128);
        assert_eq!(data[9], 64);
        assert_eq!(data[10], 32);
        assert_eq!(data[11], 255);
    }
}
