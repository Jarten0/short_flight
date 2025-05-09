use super::NPC;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// the direction the entity is facing
#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, Deref)]
pub struct FacingDirection(pub Dir2);

impl FacingDirection {
    pub fn set(&mut self, dir: Dir2) {
        self.0 = dir;
    }
}

impl Default for FacingDirection {
    fn default() -> Self {
        Self(Dir2::SOUTH)
    }
}

/// When it reaches 0, the entity will be despawned.
///
/// If death is not handled, then it automatically despawns.
#[derive(Debug, Default, Component, Reflect, Serialize, Deserialize, Clone)]
#[require(NPC)]
pub struct Health {
    pub hp: i64,
    /// can be set to true, but can never be set to false beyond initialization.
    /// thus code can always be certain that if they set it to be true,
    /// that they dont have to double check afterwards.
    #[serde(skip)]
    do_not_despawn_on_faint: bool,
    #[serde(skip)]
    currently_handling: bool,
}

impl Health {
    pub fn new(hp: i64) -> Self {
        Self {
            hp,
            do_not_despawn_on_faint: false,
            currently_handling: false,
        }
    }
}

impl std::ops::Deref for Health {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.hp
    }
}

/// Event is sent when an entity reaches 0 hp.
///
/// Should be accounted for when writing enemy AI logic.
#[derive(Debug, Event)]
pub struct OnDead {
    entity: Entity,
}

#[derive(Debug, Component)]
pub struct Dead;

pub(crate) fn query_dead(mut commands: Commands, mut query: Query<(Entity, &mut Health)>) {
    for (entity, health) in &mut query {
        if health.hp <= 0 {
            commands.send_event(OnDead { entity });
            commands.entity(entity).insert(Dead);
        }
    }
}

pub(crate) fn remove_dead(mut commands: Commands, query: Query<Entity, With<Dead>>) {
    for entity in query {
        commands.entity(entity).despawn();
    }
}

/// Multiplies with the power of the attack to increase the damage dealt.
#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, Default, Deref)]
#[require(NPC)]
pub struct Damage(pub i64);
