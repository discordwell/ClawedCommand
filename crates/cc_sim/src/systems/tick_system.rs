use bevy::prelude::*;

use crate::resources::{SimClock, VoiceOverride};

pub fn tick_system(mut clock: ResMut<SimClock>, mut voice_override: ResMut<VoiceOverride>) {
    clock.tick += 1;
    voice_override.tick();
}
