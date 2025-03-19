use bevy::prelude::*;
use short_flight::collision::Collider;

pub mod stats;

/// Marks this entity as an in-world NPC, that can interact with the player and the world via
/// collision, player interact, contact damage,
/// and can perform actions via NPC AI.
#[derive(Component)]
#[require(NPCInfo)]
pub struct NPC;

/// Marks what kind of NPC this is
#[derive(Component, Default)]
#[require(NPC)]
pub enum NPCInfo {
    /// This NPC does no kind of in-world interaction.
    #[default]
    None,
    /// This NPC can be collided with, but does nothing.
    Silent,
    /// This NPC can deal damage to the player
    Enemy {},
    Team,
}

pub struct SpawnNPC {
    pub collider: Option<Collider>,
    pub npc_info: NPCInfo,
}

impl Command for SpawnNPC {
    fn apply(self, world: &mut World) {
        match self.collider {
            Some(s) => world.spawn((NPC, self.npc_info, s)),
            None => world.spawn((NPC, self.npc_info)),
        };
    }
}
