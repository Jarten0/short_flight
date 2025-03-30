use bevy::prelude::*;
use bevy::reflect::Enum;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use bevy_asset_loader::mapped::MapKey;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

use crate::assets::{RonAssetLoader, ShortFlightLoadingState};

pub mod ai;
pub mod animation;
pub mod commands;
pub mod file;
pub mod stats;

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, stats::query_dead)
            .add_systems(
                OnExit(ShortFlightLoadingState::LoadNPCAssets),
                file::validate_npc_data,
            )
            .add_systems(PreUpdate, animation::update_sprite_timer)
            .add_systems(PostUpdate, animation::update_npc_sprites);
    }
}

/// Marks this entity as an in-world NPC, that can interact with the player and the world via
/// collision, player interact, contact damage,
/// and can perform actions via NPC AI.
#[derive(
    Component, Default, Reflect, Sequence, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Hash,
)]
pub enum NPC {
    /// npc missing identifier
    #[default]
    Void = 0,
    Geodude,
}

impl MapKey for NPC {
    fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
        let binding = std::path::PathBuf::from(
            path.path()
                .file_stem()
                .unwrap_or_else(|| panic!("Could not get the file stem for {}", path))
                .to_str()
                .unwrap_or_else(|| panic!("Could not convert {} to unicode", path)),
        );
        let stem = binding
            .file_stem()
            .unwrap_or_else(|| panic!("Could not get the file stem for {}", path))
            .to_str()
            .unwrap_or_else(|| panic!("Could not convert {} to unicode", path));
        enum_iterator::all::<Self>()
            .find(|variant| variant.variant_name() == stem)
            .unwrap_or_else(|| panic!("Could not find an NPC variant for {} [path:{}]", stem, path))
    }
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
