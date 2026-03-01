use bevy::prelude::*;

use cc_core::abilities::AbilityId;
use cc_core::building_stats::building_stats;
use cc_core::commands::EntityId;
use cc_core::components::{
    AbilitySlots, Building, Dead, HairballObstacle, Health, NineLivesTracker, Owner, UnitType,
    VisibleThroughFog,
};
use cc_core::math::FIXED_ZERO;
use cc_core::status_effects::StatusEffectId;
use cc_core::tuning::{
    NINE_LIVES_COOLDOWN_TICKS, NINE_LIVES_GPU_COST, NINE_LIVES_HP_FRACTION,
    NINE_LIVES_REVIVE_TICKS,
};
use cc_core::unit_stats::base_stats;

use crate::resources::{PlayerResources, SimClock};
use crate::systems::damage::ApplyStatusCommand;

/// Mark units with zero health as Dead, with NineLives intercept for Chonk revive.
/// Also reclaims supply/supply_cap and ticks down hairball/VTF timers.
pub fn cleanup_system(
    mut commands: Commands,
    mut newly_dead: Query<
        (
            Entity,
            &mut Health,
            &Owner,
            Option<&UnitType>,
            Option<&Building>,
            Option<&AbilitySlots>,
            Option<&mut NineLivesTracker>,
        ),
        Without<Dead>,
    >,
    mut player_resources: ResMut<PlayerResources>,
    clock: Res<SimClock>,
    mut hairballs: Query<(Entity, &mut HairballObstacle)>,
    mut vis_fog: Query<(Entity, &mut VisibleThroughFog)>,
) {
    for (entity, mut health, owner, unit_type, building, slots, nine_lives) in
        newly_dead.iter_mut()
    {
        if health.current <= FIXED_ZERO {
            // Check NineLives passive for Chonk revive
            let revived = if let (Some(mut tracker), Some(slots)) = (nine_lives, slots) {
                let has_nine_lives = slots.slots.iter().any(|s| s.id == AbilityId::NineLives);
                let cooldown_ok = tracker.last_triggered_tick == 0
                    || clock
                        .tick
                        .saturating_sub(tracker.last_triggered_tick)
                        >= NINE_LIVES_COOLDOWN_TICKS;
                let can_afford = player_resources
                    .players
                    .get(owner.player_id as usize)
                    .is_some_and(|p| p.gpu_cores >= NINE_LIVES_GPU_COST);

                if has_nine_lives && cooldown_ok && can_afford {
                    health.current = health.max * NINE_LIVES_HP_FRACTION;
                    tracker.last_triggered_tick = clock.tick;
                    if let Some(pres) =
                        player_resources.players.get_mut(owner.player_id as usize)
                    {
                        pres.gpu_cores -= NINE_LIVES_GPU_COST;
                    }
                    commands.queue(ApplyStatusCommand {
                        target: entity,
                        effect: StatusEffectId::NineLivesReviving,
                        duration: NINE_LIVES_REVIVE_TICKS,
                        stacks: 1,
                        max_stacks: 1,
                        source: EntityId::from_entity(entity),
                    });
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if !revived {
                commands.entity(entity).insert(Dead);

                let player_id = owner.player_id as usize;
                if let Some(pres) = player_resources.players.get_mut(player_id) {
                    if let Some(ut) = unit_type {
                        let supply_cost = base_stats(ut.kind).supply_cost;
                        pres.supply = pres.supply.saturating_sub(supply_cost);
                    }
                    if let Some(bld) = building {
                        let supply_provided = building_stats(bld.kind).supply_provided;
                        if supply_provided > 0 {
                            pres.supply_cap = pres.supply_cap.saturating_sub(supply_provided);
                            pres.supply = pres.supply.min(pres.supply_cap);
                        }
                    }
                }
            }
        }
    }

    for (entity, mut hairball) in hairballs.iter_mut() {
        hairball.remaining_ticks = hairball.remaining_ticks.saturating_sub(1);
        if hairball.remaining_ticks == 0 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut vis) in vis_fog.iter_mut() {
        vis.remaining_ticks = vis.remaining_ticks.saturating_sub(1);
        if vis.remaining_ticks == 0 {
            commands.entity(entity).remove::<VisibleThroughFog>();
        }
    }
}
