use bevy::prelude::*;

use cc_core::components::{Building, BuildingKind, Dead, Owner};

use crate::resources::{GameState, PlayerResources};

/// Check win condition: if a player's TheBox is destroyed, the other player wins.
/// Supports any number of players — the last player with a living TheBox wins.
pub fn victory_system(
    mut game_state: ResMut<GameState>,
    player_resources: Res<PlayerResources>,
    buildings: Query<(&Building, &Owner), Without<Dead>>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    let total_players = player_resources.players.len();
    if total_players < 2 {
        return;
    }

    // Collect which players still have a living TheBox
    let mut players_with_box: Vec<u8> = Vec::new();
    for (building, owner) in buildings.iter() {
        if building.kind == BuildingKind::TheBox && !players_with_box.contains(&owner.player_id) {
            players_with_box.push(owner.player_id);
        }
    }

    // If exactly one player has a TheBox remaining, they win
    if players_with_box.len() == 1 {
        *game_state = GameState::Victory {
            winner: players_with_box[0],
        };
    } else if players_with_box.is_empty() {
        // All boxes destroyed simultaneously — attacker advantage tiebreak (player 0 wins)
        *game_state = GameState::Victory { winner: 0 };
    }
}
