use bevy::prelude::*;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::setup::UnitMesh;
use cc_core::components::{Owner, Position};
use cc_core::terrain::TerrainType;
use cc_sim::resources::MapResource;

/// Timer-gated minimap refresh (every 0.3s).
#[derive(Resource)]
pub struct MinimapTimer(pub Timer);

/// Handle to the minimap image.
#[derive(Resource)]
pub struct MinimapImage(pub Handle<Image>);

/// Marker for the minimap UI node.
#[derive(Component)]
pub struct MinimapNode;

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
        let map = &map_res.map;
        for y in 0..(h as usize) {
            for x in 0..(w as usize) {
                let grid = cc_core::coords::GridPos::new(x as i32, y as i32);
                if let Some(tile) = map.get(grid) {
                    let (r, g, b) = minimap_terrain_color(tile.terrain);
                    let idx = (y * w as usize + x) * 4;
                    data[idx] = r;
                    data[idx + 1] = g;
                    data[idx + 2] = b;
                    data[idx + 3] = 255;
                }
            }
        }
    }

    let image_handle = images.add(image);

    commands.insert_resource(MinimapImage(image_handle.clone()));
    commands.insert_resource(MinimapTimer(Timer::from_seconds(0.3, TimerMode::Repeating)));

    // UI node: bottom-left corner
    commands
        .spawn((
            MinimapNode,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(200.0),
                height: Val::Px(200.0),
                border: UiRect::all(Val::Px(2.0)),
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

/// Update the minimap image with terrain colors and unit dots.
pub fn update_minimap(
    time: Res<Time>,
    mut timer: ResMut<MinimapTimer>,
    minimap_img: Option<Res<MinimapImage>>,
    mut images: ResMut<Assets<Image>>,
    map_res: Res<MapResource>,
    units: Query<(&Position, &Owner), With<UnitMesh>>,
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

    // Write terrain colors
    for y in 0..h {
        for x in 0..w {
            let grid = cc_core::coords::GridPos::new(x as i32, y as i32);
            let tile = map.get(grid).unwrap();
            let (r, g, b) = minimap_terrain_color(tile.terrain);
            let idx = (y * w + x) * 4;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    // Write unit dots
    for (pos, owner) in units.iter() {
        let grid = pos.world.to_grid();
        if grid.x >= 0 && grid.y >= 0 && (grid.x as u32) < map.width && (grid.y as u32) < map.height
        {
            let idx = (grid.y as usize * w + grid.x as usize) * 4;
            let (r, g, b) = if owner.player_id == 0 {
                (50, 100, 230) // Blue
            } else {
                (230, 50, 50) // Red
            };
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }
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
    }
}
