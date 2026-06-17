// Bevy ECS queries and systems naturally exceed these thresholds
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

/// Local player ID (TODO: make configurable for multiplayer).
pub const LOCAL_PLAYER: u8 = 0;

pub mod cutscene;
pub mod dream;
pub mod dream_strait;
pub mod dream_test;
pub mod input;
pub mod loading;
pub mod renderer;
pub mod setup;
pub mod showcase;
pub mod ui;
pub mod voice_demo;
