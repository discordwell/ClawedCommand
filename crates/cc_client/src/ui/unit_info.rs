use bevy::prelude::*;

use cc_core::components::{
    AttackStats, Building, Health, ProductionQueue, RallyPoint, Selected, UnderConstruction,
    UnitType,
};
use cc_core::unit_stats::base_stats;

/// Marker for the unit info panel root.
#[derive(Component)]
pub struct UnitInfoRoot;

/// Marker for the unit info text.
#[derive(Component)]
pub struct UnitInfoText;

pub fn spawn_unit_info(mut commands: Commands) {
    commands
        .spawn((
            UnitInfoRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(28.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                UnitInfoText,
                Text::new("No selection"),
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        });
}

pub fn update_unit_info(
    selected_units: Query<(&UnitType, &Health, &AttackStats), With<Selected>>,
    selected_buildings: Query<
        (
            &Building,
            &Health,
            Option<&UnderConstruction>,
            Option<&ProductionQueue>,
            Option<&RallyPoint>,
        ),
        With<Selected>,
    >,
    mut text_q: Query<(&mut Text, &mut TextColor), With<UnitInfoText>>,
) {
    let Ok((mut text, mut color)) = text_q.single_mut() else {
        return;
    };

    let unit_count = selected_units.iter().count();
    let building_count = selected_buildings.iter().count();

    if unit_count == 0 && building_count == 0 {
        text.0 = "No selection".to_string();
        color.0 = Color::srgb(0.5, 0.5, 0.5);
        return;
    }

    color.0 = Color::srgb(0.9, 0.9, 0.9);

    // Single unit
    if unit_count == 1 && building_count == 0 {
        if let Ok((unit_type, health, attack)) = selected_units.single() {
            let hp_cur: f32 = health.current.to_num();
            let hp_max: f32 = health.max.to_num();
            let dmg: f32 = attack.damage.to_num();
            let rng: f32 = attack.range.to_num();
            text.0 = format!(
                "{:?}  |  HP: {:.0}/{:.0}  |  ATK: {:.0}  |  RNG: {:.1}",
                unit_type.kind, hp_cur, hp_max, dmg, rng
            );
        }
        return;
    }

    // Single building
    if building_count == 1 && unit_count == 0 {
        if let Ok((building, health, uc, queue, rally)) = selected_buildings.single() {
            let hp_cur: f32 = health.current.to_num();
            let hp_max: f32 = health.max.to_num();
            let mut parts = vec![format!("{:?}  |  HP: {:.0}/{:.0}", building.kind, hp_cur, hp_max)];

            if let Some(uc) = uc {
                let progress = if uc.total_ticks > 0 {
                    1.0 - (uc.remaining_ticks as f32 / uc.total_ticks as f32)
                } else {
                    1.0
                };
                parts.push(format!("Building... {:.0}%", progress * 100.0));
            } else if let Some(queue) = queue {
                if let Some((kind, ticks_remaining)) = queue.queue.front() {
                    let stats = base_stats(*kind);
                    let total_secs = stats.train_time as f32 / 10.0;
                    let remaining_secs = *ticks_remaining as f32 / 10.0;
                    let elapsed_secs = (total_secs - remaining_secs).max(0.0);
                    parts.push(format!("Training: {:?} {:.0}/{:.0}s", kind, elapsed_secs, total_secs));
                    let queued = queue.queue.len() - 1;
                    if queued > 0 {
                        parts.push(format!("+{} queued", queued));
                    }
                } else {
                    parts.push("Idle".to_string());
                }
            }

            if let Some(rally) = rally {
                parts.push(format!("Rally: ({},{})", rally.target.x, rally.target.y));
            }

            text.0 = parts.join("  |  ");
        }
        return;
    }

    // Multi-select
    if unit_count > 1 && building_count == 0 {
        use std::collections::HashMap;
        let mut type_counts: HashMap<cc_core::components::UnitKind, u32> = HashMap::new();
        for (unit_type, _, _) in selected_units.iter() {
            *type_counts.entry(unit_type.kind).or_insert(0) += 1;
        }
        let mut parts: Vec<String> = type_counts
            .iter()
            .map(|(kind, count)| format!("{}x {:?}", count, kind))
            .collect();
        parts.sort();
        text.0 = format!("{} selected: {}", unit_count, parts.join(", "));
    } else {
        let mut label_parts = Vec::new();
        if unit_count > 0 {
            label_parts.push(format!(
                "{} unit{}",
                unit_count,
                if unit_count > 1 { "s" } else { "" }
            ));
        }
        if building_count > 0 {
            label_parts.push(format!(
                "{} building{}",
                building_count,
                if building_count > 1 { "s" } else { "" }
            ));
        }
        text.0 = label_parts.join(" + ");
    }
}
