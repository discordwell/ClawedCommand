use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::components::{
    Building, BuildingKind, Owner, Producer, ProductionQueue, Selected, UnderConstruction,
};
use cc_core::unit_stats::base_stats;
use cc_sim::resources::PlayerResources;

/// Marker for the building info panel root node.
#[derive(Component)]
pub struct BuildingInfoRoot;

/// Marker for the building info text content.
#[derive(Component)]
pub struct BuildingInfoText;

pub fn spawn_building_info(mut commands: Commands) {
    commands
        .spawn((
            BuildingInfoRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(8.0),
                right: Val::Px(8.0),
                width: Val::Px(260.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                BuildingInfoText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        });
}

pub fn update_building_info(
    player_resources: Res<PlayerResources>,
    selected_buildings: Query<
        (
            &Building,
            &Owner,
            Option<&UnderConstruction>,
            Option<&ProductionQueue>,
            Option<&Producer>,
        ),
        With<Selected>,
    >,
    mut root_q: Query<&mut Visibility, (With<BuildingInfoRoot>, Without<BuildingInfoText>)>,
    mut text_q: Query<&mut Text, (With<BuildingInfoText>, Without<BuildingInfoRoot>)>,
) {
    // Find first selected building
    let selected = selected_buildings.iter().next();

    let show = selected.is_some();
    for mut vis in root_q.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    let Some((building, owner, uc, queue, producer)) = selected else {
        return;
    };

    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let name = building_display_name(building.kind);
    let bstats = building_stats(building.kind);
    let pres = player_resources.players.get(owner.player_id as usize);

    let mut lines = Vec::new();

    // Under construction
    if let Some(uc) = uc {
        let progress = if uc.total_ticks > 0 {
            ((uc.total_ticks - uc.remaining_ticks) as f32 / uc.total_ticks as f32 * 100.0) as u32
        } else {
            100
        };
        let filled = (progress as usize) / 10;
        let bar: String = "\u{2588}".repeat(filled)
            + &"\u{2591}".repeat(10 - filled);
        lines.push(format!("{name} (Building...)"));
        lines.push(format!("{bar} {progress}%"));
    } else if producer.is_some() {
        // Completed producer building
        lines.push(name.to_string());

        let keys = ["Q", "W", "E", "R"];
        for (i, &unit_kind) in bstats.can_produce.iter().enumerate() {
            let key = keys.get(i).unwrap_or(&"?");
            let ustats = base_stats(unit_kind);
            let mut cost_parts = vec![format!("{}f", ustats.food_cost)];
            if ustats.gpu_cost > 0 {
                cost_parts.push(format!("{}g", ustats.gpu_cost));
            }
            let cost_str = cost_parts.join(" ");

            let affordable = if let Some(p) = pres {
                p.food >= ustats.food_cost && p.gpu_cores >= ustats.gpu_cost
            } else {
                false
            };
            let marker = if affordable { " " } else { "*" };
            lines.push(format!(
                "[{key}] {kind:<12} {cost:<10} {sup}sup{marker}",
                kind = format!("{unit_kind:?}"),
                cost = cost_str,
                sup = ustats.supply_cost,
            ));
        }

        // Show queue
        if let Some(q) = queue {
            if !q.queue.is_empty() {
                let queue_items: Vec<String> = q
                    .queue
                    .iter()
                    .map(|(kind, ticks)| {
                        let secs = *ticks as f32 / 10.0;
                        format!("{kind:?}({secs:.0}s)")
                    })
                    .collect();
                lines.push(format!("Queue: {}", queue_items.join(" > ")));
            }
        }
    } else {
        // Non-producer building (FishMarket, LitterBox, etc.)
        lines.push(name.to_string());
        if bstats.supply_provided > 0 {
            lines.push(format!("Supply: +{}", bstats.supply_provided));
        }
        match building.kind {
            BuildingKind::FishMarket => lines.push("Resource drop-off".to_string()),
            BuildingKind::CatFlap => lines.push("Garrison building".to_string()),
            BuildingKind::LaserPointer => lines.push("Defensive tower".to_string()),
            BuildingKind::ScratchingPost => lines.push("Research building".to_string()),
            _ => {}
        }
    }

    text.0 = lines.join("\n");
}

fn building_display_name(kind: BuildingKind) -> &'static str {
    match kind {
        BuildingKind::TheBox => "The Box",
        BuildingKind::CatTree => "Cat Tree",
        BuildingKind::FishMarket => "Fish Market",
        BuildingKind::LitterBox => "Litter Box",
        BuildingKind::ServerRack => "Server Rack",
        BuildingKind::ScratchingPost => "Scratching Post",
        BuildingKind::CatFlap => "Cat Flap",
        BuildingKind::LaserPointer => "Laser Pointer",
    }
}
