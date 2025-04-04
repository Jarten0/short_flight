use crate::assets::{AnimationSpritesheet, RonAssetLoader};
use crate::npc::animation::NPCAnimation;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::mapped::MapKey;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

/// Marks this entity as a move, aka an attack, that temporarily exists in the world.
#[derive(
    Component,
    Default,
    Reflect,
    Sequence,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
)]
pub enum Move {
    #[default]
    Void = 0,
    Tackle,
    MagicalLeaf,
}

fn void() {}
fn tackle(query: Query<&NPCAnimation>) {}

fn register_systems(app: &mut App, input: Move) {
    match input {
        Move::Void => app.add_systems(FixedUpdate, void),
        Move::Tackle => app.add_systems(FixedUpdate, tackle),
        Move::MagicalLeaf => todo!(),
    };
}

pub struct MovePlugin;

impl Plugin for MovePlugin {
    fn build(&self, app: &mut App) {
        for input in enum_iterator::all::<Move>() {
            register_systems(app, input)
        }
    }
}

#[derive(Resource, AssetCollection)]
pub(crate) struct MoveList {
    #[asset(path = "move_data", collection(typed, mapped))]
    pub data_files: HashMap<Move, Handle<MoveData>>,

    /// An associated texture
    #[asset(path = "moves", collection(typed, mapped))]
    pub image_files: HashMap<Move, Handle<Image>>,
}

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct MoveData {
    pub(crate) display_name: String,
    pub(crate) spritesheet: AnimationSpritesheet,
}

impl MapKey for Move {
    fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
        short_flight::from_asset_path(path)
    }
}
