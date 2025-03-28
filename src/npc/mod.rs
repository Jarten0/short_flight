use bevy::prelude::*;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

pub mod ai;
pub mod animation;
pub mod commands;
pub mod file;
pub mod stats;

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        let init_asset = app
            .add_systems(Update, stats::query_dead)
            .add_systems(PreStartup, file::load_npcs)
            .add_systems(PreUpdate, animation::update_sprite_timer)
            .add_systems(PostUpdate, animation::update_npc_sprites)
            .init_asset::<file::NPCData>();
    }
}

/// Marks this entity as an in-world NPC, that can interact with the player and the world via
/// collision, player interact, contact damage,
/// and can perform actions via NPC AI.
#[derive(Component, Default, Reflect, Clone, Copy, PartialEq, Eq, PartialOrd, Sequence, Hash)]
pub enum NPC {
    /// npc missing identifier
    #[default]
    Void,
    Geodude,
}

impl TryFrom<usize> for NPC {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > enum_iterator::cardinality::<NPC>() {
            return Err(());
        }
        Ok(enum_iterator::all::<NPC>().nth(value).unwrap())
    }
}

/// Marks what kind of NPC this is
#[derive(Debug, Component, Default, Reflect, Clone, Serialize, Deserialize)]
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
