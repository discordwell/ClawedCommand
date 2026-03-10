use bevy::prelude::*;

use cc_core::components::{Dead, Position, StationaryTimer};

/// Tracks how long each unit has been stationary by comparing positions each tick.
/// Used by combat_system's anti-static damage bonus.
pub fn stationary_timer_system(mut query: Query<(&Position, &mut StationaryTimer), Without<Dead>>) {
    for (pos, mut timer) in query.iter_mut() {
        let moved = timer.last_pos.map_or(false, |last| last != pos.world);

        if moved {
            timer.ticks_stationary = 0;
        } else {
            timer.ticks_stationary = timer.ticks_stationary.saturating_add(1);
        }

        // Seekers Dug In bonus at 50 ticks (5s)
        timer.dug_in = timer.ticks_stationary >= 50;

        timer.last_pos = Some(pos.world);
    }
}
