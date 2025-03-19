use bevy::prelude::*;

/// When it reaches 0, the entity will be despawned.
///
/// If death is not handled, then it automatically despawns.
#[derive(Debug, Default, Component, Reflect)]
#[require(NPC)]
pub struct Health {
    pub hp: u64,
    /// can be set to true, but can never be set to false beyond initialization.
    /// thus code can always be certain that if they set it to be true,
    /// that they dont have to double check afterwards.
    do_not_despawn_on_faint: bool,
    currently_handling: bool,
}

/// Event is sent when an entity reaches 0 hp.
///
#[derive(Debug, Event)]
pub struct OnDead;

fn query_dead() {}
