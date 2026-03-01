use bevy::prelude::*;

use cc_core::components::{Building, Dead, Owner};

use crate::resources::{GameState, PlayerResources};

/// Check win condition: if a player's HQ is destroyed, the other player wins.
/// Supports any number of players — the last player with a living HQ wins.
/// Recognizes all faction HQs (TheBox, TheParliament, TheBurrow, TheSett, TheGrotto, TheDumpster).
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

    // Collect which players still have a living HQ
    let mut players_with_hq: Vec<u8> = Vec::new();
    for (building, owner) in buildings.iter() {
        if building.kind.is_hq() && !players_with_hq.contains(&owner.player_id) {
            players_with_hq.push(owner.player_id);
        }
    }

    // If exactly one player has an HQ remaining, they win
    if players_with_hq.len() == 1 {
        *game_state = GameState::Victory {
            winner: players_with_hq[0],
        };
    }
    // If all HQs destroyed simultaneously, don't declare a winner —
    // let the game continue until one side has no living entities.
}
