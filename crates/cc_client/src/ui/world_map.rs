use bevy::prelude::*;

use cc_core::mission::{MissionDefinition, NextMission};
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

use super::campaign_menu::AvailableMissions;
use super::cinematic::{act_display_name, faction_accent_color};

/// Marker for the world map root overlay.
#[derive(Component)]
pub struct WorldMapRoot;

/// Marker for a mission node button.
#[derive(Component)]
pub struct MissionNode {
    pub mission_id: String,
}

/// Marker for the back button.
#[derive(Component)]
pub struct WorldMapBackButton;

/// Marker for the progress text.
#[derive(Component)]
pub struct WorldMapProgress;

/// Marker for the world map content area (to track if spawned).
#[derive(Component)]
pub struct WorldMapContent;

/// Marker for act column entities (cleaned up on rebuild).
#[derive(Component)]
pub struct ActColumn;

pub fn spawn_world_map(mut commands: Commands) {
    commands
        .spawn((
            WorldMapRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.06, 0.96)),
            Visibility::Hidden,
            ZIndex(90),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("CAMPAIGN"),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ));

            // Scrollable mission grid area
            parent
                .spawn((
                    WorldMapContent,
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(24.0),
                        flex_grow: 1.0,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                ));

            // Bottom bar
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    width: Val::Percent(100.0),
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|bar| {
                    // Back button
                    bar.spawn((
                        WorldMapBackButton,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("BACK TO MENU"),
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                        ));
                    });

                    // Progress text
                    bar.spawn((
                        WorldMapProgress,
                        Text::new(""),
                        TextColor(Color::srgb(0.6, 0.6, 0.6)),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                    ));
                });
        });
}

/// Returns true if a mission is unlocked (prologue always unlocked, or any
/// predecessor completed).
fn is_unlocked(mission_id: &str, all_missions: &[MissionDefinition], campaign: &CampaignState) -> bool {
    // Prologue is always unlocked
    if mission_id == "prologue" {
        return true;
    }

    // A mission is unlocked if any mission that has it as next_mission is completed
    for m in all_missions {
        let leads_here = match &m.next_mission {
            NextMission::Fixed(id) => id == mission_id,
            NextMission::Branching {
                on_true, on_false, ..
            } => on_true == mission_id || on_false == mission_id,
            NextMission::None => false,
        };
        if leads_here && campaign.completed_missions.contains(&m.id) {
            return true;
        }
    }

    false
}

/// Rebuild world map nodes when missions change.
pub fn update_world_map(
    mut commands: Commands,
    campaign: Res<CampaignState>,
    available: Res<AvailableMissions>,
    mut root_vis: Query<&mut Visibility, (With<WorldMapRoot>, Without<WorldMapProgress>)>,
    content_q: Query<Entity, With<WorldMapContent>>,
    existing_nodes: Query<Entity, Or<(With<MissionNode>, With<ActColumn>)>>,
    mut progress_q: Query<&mut Text, With<WorldMapProgress>>,
) {
    let show = campaign.phase == CampaignPhase::WorldMap;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        return;
    }

    // Update progress counter
    let total = available.missions.len();
    let completed = campaign.completed_missions.len();
    if let Ok(mut text) = progress_q.single_mut() {
        text.0 = format!("{completed}/{total} missions completed");
    }

    // Only rebuild nodes when missions resource changes
    if !available.is_changed() && !campaign.is_changed() {
        return;
    }

    // Clear existing nodes and act columns (despawn includes children in Bevy 0.18)
    for entity in existing_nodes.iter() {
        commands.entity(entity).despawn();
    }

    let Ok(content_entity) = content_q.single() else {
        return;
    };

    // Group missions by act
    let mut acts: Vec<(u32, Vec<&MissionDefinition>)> = Vec::new();
    for mission in &available.missions {
        if let Some((_act, missions)) = acts.iter_mut().find(|(a, _)| *a == mission.act) {
            missions.push(mission);
        } else {
            acts.push((mission.act, vec![mission]));
        }
    }
    acts.sort_by_key(|(a, _)| *a);

    // Spawn act columns
    for (act, missions) in &acts {
        let accent = faction_accent_color(*act);

        let col_entity = commands
            .spawn((ActColumn, Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                min_width: Val::Px(120.0),
                ..default()
            }))
            .with_children(|col| {
                // Act header
                col.spawn((
                    Text::new(act_display_name(*act)),
                    TextColor(accent),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Mission nodes
                let mut sorted_missions: Vec<&&MissionDefinition> = missions.iter().collect();
                sorted_missions.sort_by_key(|m| m.mission_index);

                for mission in sorted_missions {
                    let unlocked =
                        is_unlocked(&mission.id, &available.missions, &campaign);
                    let completed = campaign.completed_missions.contains(&mission.id);

                    let (bg_color, border_color, label_color) = if completed {
                        (
                            Color::srgba(0.1, 0.3, 0.1, 0.8),
                            accent,
                            Color::srgb(0.8, 0.8, 0.8),
                        )
                    } else if unlocked {
                        (
                            Color::srgba(0.15, 0.15, 0.2, 0.8),
                            accent,
                            Color::srgb(0.9, 0.9, 0.9),
                        )
                    } else {
                        (
                            Color::srgba(0.1, 0.1, 0.1, 0.6),
                            Color::srgba(0.3, 0.3, 0.3, 0.5),
                            Color::srgb(0.4, 0.4, 0.4),
                        )
                    };

                    let status_icon = if completed {
                        "[*] "
                    } else if unlocked {
                        "[ ] "
                    } else {
                        "[X] "
                    };

                    col.spawn((
                        MissionNode {
                            mission_id: mission.id.clone(),
                        },
                        Button,
                        Node {
                            width: Val::Px(110.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(bg_color),
                        BorderColor::all(border_color),
                    ))
                    .with_children(|node| {
                        node.spawn((
                            Text::new(format!("{}{}", status_icon, mission.name)),
                            TextColor(label_color),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                        ));
                    });
                }
            })
            .id();

        commands.entity(content_entity).add_child(col_entity);
    }
}

/// Handle world map interactions — click mission node to load it.
pub fn world_map_interaction(
    mut campaign: ResMut<CampaignState>,
    available: Res<AvailableMissions>,
    node_interactions: Query<(&MissionNode, &Interaction), Changed<Interaction>>,
    back_interactions: Query<(&WorldMapBackButton, &Interaction), Changed<Interaction>>,
) {
    // Back button
    for (_, interaction) in back_interactions.iter() {
        if *interaction == Interaction::Pressed {
            campaign.phase = CampaignPhase::Inactive;
        }
    }

    // Mission node clicks
    for (node, interaction) in node_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Check if unlocked
        if !is_unlocked(&node.mission_id, &available.missions, &campaign) {
            continue;
        }

        // Load mission RON
        let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/campaign")
            .join(format!("{}.ron", node.mission_id));

        match std::fs::read_to_string(&ron_path) {
            Ok(ron_str) => {
                match ron::from_str::<MissionDefinition>(&ron_str) {
                    Ok(mission) => {
                        let new_act = mission.act;
                        let old_act = campaign
                            .current_mission
                            .as_ref()
                            .map(|m| m.act);

                        campaign.load_mission(mission);

                        // Show act title card if entering a new act
                        if old_act != Some(new_act) {
                            campaign.entering_act = Some(new_act);
                            campaign.phase = CampaignPhase::ActTitleCard;
                        }
                        // else stays in Briefing (set by load_mission)
                    }
                    Err(e) => {
                        warn!("Failed to parse {}.ron: {e}", node.mission_id);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read {}.ron: {e}", node.mission_id);
            }
        }
    }
}

/// Handle Escape key to go back from world map.
pub fn world_map_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut campaign: ResMut<CampaignState>,
) {
    if campaign.phase != CampaignPhase::WorldMap {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        campaign.phase = CampaignPhase::Inactive;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::mission::*;
    use cc_core::terrain::TerrainType;
    use cc_core::hero::HeroId;
    use cc_core::coords::GridPos;

    fn minimal_mission(id: &str, act: u32, next: NextMission) -> MissionDefinition {
        MissionDefinition {
            id: id.into(),
            name: id.into(),
            act,
            mission_index: 0,
            map: MissionMap::Inline {
                width: 2,
                height: 2,
                tiles: vec![TerrainType::Grass; 4],
                elevation: vec![0; 4],
            },
            player_setup: PlayerSetup {
                heroes: vec![HeroSpawn {
                    hero_id: HeroId::Kelpie,
                    position: GridPos::new(0, 0),
                    mission_critical: false,
                    player_id: 0,
                }],
                units: vec![],
                buildings: vec![],
                starting_food: 0,
                starting_gpu: 0,
                starting_nfts: 0,
            },
            enemy_waves: vec![],
            objectives: vec![],
            triggers: vec![],
            dialogue: vec![],
            briefing_text: "".into(),
            debrief_text: "".into(),
            ai_tool_tier: None,
            next_mission: next,
            mutators: vec![],
        }
    }

    #[test]
    fn prologue_always_unlocked() {
        let missions = vec![minimal_mission("prologue", 0, NextMission::Fixed("m1".into()))];
        let campaign = CampaignState::default();
        assert!(is_unlocked("prologue", &missions, &campaign));
    }

    #[test]
    fn mission_unlocked_after_predecessor_completed() {
        let missions = vec![
            minimal_mission("prologue", 0, NextMission::Fixed("m1".into())),
            minimal_mission("m1", 1, NextMission::None),
        ];
        let mut campaign = CampaignState::default();
        assert!(!is_unlocked("m1", &missions, &campaign));

        campaign.completed_missions.insert("prologue".into());
        assert!(is_unlocked("m1", &missions, &campaign));
    }

    #[test]
    fn branching_unlock_works() {
        let missions = vec![
            minimal_mission(
                "m12",
                3,
                NextMission::Branching {
                    flag: "helped_rex".into(),
                    on_true: "m13a".into(),
                    on_false: "m13b".into(),
                },
            ),
            minimal_mission("m13a", 3, NextMission::None),
            minimal_mission("m13b", 3, NextMission::None),
        ];
        let mut campaign = CampaignState::default();
        campaign.completed_missions.insert("m12".into());
        assert!(is_unlocked("m13a", &missions, &campaign));
        assert!(is_unlocked("m13b", &missions, &campaign));
    }

    #[test]
    fn locked_mission_stays_locked() {
        let missions = vec![
            minimal_mission("prologue", 0, NextMission::Fixed("m1".into())),
            minimal_mission("m1", 1, NextMission::Fixed("m2".into())),
            minimal_mission("m2", 1, NextMission::None),
        ];
        let mut campaign = CampaignState::default();
        campaign.completed_missions.insert("prologue".into());
        // m1 is unlocked, but m2 is NOT (m1 not completed)
        assert!(is_unlocked("m1", &missions, &campaign));
        assert!(!is_unlocked("m2", &missions, &campaign));
    }
}
