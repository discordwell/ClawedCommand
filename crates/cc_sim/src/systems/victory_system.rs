use bevy::prelude::*;

use cc_core::components::{Building, BuildingKind, Dead, Owner};

use crate::resources::GameState;

/// Check win condition: if a player's TheBox is destroyed, the other player wins.
pub fn victory_system(
    mut game_state: ResMut<GameState>,
    buildings: Query<(&Building, &Owner), Without<Dead>>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    // Check which players still have a living TheBox
    let mut has_box = [false; 2];
    for (building, owner) in buildings.iter() {
        if building.kind == BuildingKind::TheBox && (owner.player_id as usize) < 2 {
            has_box[owner.player_id as usize] = true;
        }
    }

    // If player 0's box is gone, player 1 wins (and vice versa)
    if !has_box[0] && has_box[1] {
        *game_state = GameState::Victory { winner: 1 };
    } else if has_box[0] && !has_box[1] {
        *game_state = GameState::Victory { winner: 0 };
    }
}
