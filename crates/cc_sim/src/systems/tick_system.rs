use bevy::prelude::*;

use crate::resources::SimClock;

pub fn tick_system(mut clock: ResMut<SimClock>) {
    clock.tick += 1;
}
