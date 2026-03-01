use bevy::prelude::*;

use crate::input::InputMode;
use cc_core::building_stats::building_stats;
use cc_core::components::BuildingKind;
use cc_sim::resources::PlayerResources;

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: usize = 0;

/// Marker for the build menu root node.
#[derive(Component)]
pub struct BuildMenuRoot;

/// Marker for individual build menu entry text.
#[derive(Component)]
pub struct BuildMenuEntry {
    pub kind: BuildingKind,
}

/// Marker for the placement hint text.
#[derive(Component)]
pub struct PlacementHint;

/// Build menu entries: (key label, BuildingKind)
const MENU_ENTRIES: &[(&str, BuildingKind)] = &[
    ("T", BuildingKind::CatTree),
    ("F", BuildingKind::FishMarket),
    ("L", BuildingKind::LitterBox),
    ("S", BuildingKind::ServerRack),
    ("P", BuildingKind::ScratchingPost),
    ("C", BuildingKind::CatFlap),
    ("R", BuildingKind::LaserPointer),
];

fn cost_string(kind: BuildingKind) -> String {
    let stats = building_stats(kind);
    let mut parts = vec![format!("{}f", stats.food_cost)];
    if stats.gpu_cost > 0 {
        parts.push(format!("{}g", stats.gpu_cost));
    }
    parts.join(" ")
}

fn building_description(kind: BuildingKind) -> &'static str {
    match kind {
        BuildingKind::CatTree => "barracks",
        BuildingKind::FishMarket => "drop-off",
        BuildingKind::LitterBox => "+10 supply",
        BuildingKind::ServerRack => "advanced",
        BuildingKind::ScratchingPost => "research",
        BuildingKind::CatFlap => "garrison",
        BuildingKind::LaserPointer => "tower",
        BuildingKind::TheBox => "HQ",
        _ => "building",
    }
}

pub fn spawn_build_menu(mut commands: Commands) {
    // Build menu overlay (bottom-center, hidden by default)
    commands
        .spawn((
            BuildMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(60.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-150.0)),
                width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("BUILD (select with key)"),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            // Entries
            for &(key, kind) in MENU_ENTRIES {
                let name = kind.display_name();
                let cost = cost_string(kind);
                let desc = building_description(kind);
                parent.spawn((
                    BuildMenuEntry { kind },
                    Text::new(format!("[{key}] {name:<14} {cost:<8} {desc}")),
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                ));
            }
        });

    // Placement hint (top-center, hidden by default)
    commands.spawn((
        PlacementHint,
        Text::new(""),
        TextColor(Color::srgb(0.9, 0.9, 0.5)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-150.0)),
            ..default()
        },
        Visibility::Hidden,
    ));
}

pub fn update_build_menu(
    input_mode: Res<InputMode>,
    player_resources: Res<PlayerResources>,
    mut menu_root: Query<&mut Visibility, (With<BuildMenuRoot>, Without<PlacementHint>, Without<BuildMenuEntry>)>,
    mut entries: Query<(&BuildMenuEntry, &mut TextColor), Without<BuildMenuRoot>>,
    mut hint_q: Query<(&mut Text, &mut Visibility), (With<PlacementHint>, Without<BuildMenuRoot>, Without<BuildMenuEntry>)>,
) {
    let show_menu = *input_mode == InputMode::BuildMenu;

    // Toggle build menu visibility
    for mut vis in menu_root.iter_mut() {
        *vis = if show_menu {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update entry colors based on affordability
    if show_menu {
        let pres = player_resources.players.get(LOCAL_PLAYER);
        for (entry, mut color) in entries.iter_mut() {
            let stats = building_stats(entry.kind);
            let affordable = if let Some(p) = pres {
                p.food >= stats.food_cost && p.gpu_cores >= stats.gpu_cost
            } else {
                false
            };
            color.0 = if affordable {
                Color::srgb(0.9, 0.9, 0.9)
            } else {
                Color::srgb(0.4, 0.4, 0.4)
            };
        }
    }

    // Placement hint
    if let Ok((mut text, mut vis)) = hint_q.single_mut() {
        if let InputMode::BuildPlacement { kind } = *input_mode {
            let name = format!("{kind:?}");
            text.0 = format!("Click to place {name} -- Right-click to cancel");
            *vis = Visibility::Inherited;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
