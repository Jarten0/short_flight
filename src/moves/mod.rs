use crate::assets::{AnimationSpritesheet, RonAssetLoader};
use crate::npc::animation::NPCAnimation;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::mapped::MapKey;
use bevy_sprite3d::Sprite3d;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use short_flight::collision::ColliderShape;

/// Marks this entity as a move, aka an attack, that temporarily exists in the world.
#[derive(
    Debug,
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

fn register_systems(app: &mut App, input: Move) {
    match input {
        Move::Void => app.add_systems(FixedUpdate, void),
        Move::Tackle => app.add_systems(FixedUpdate, tackle),
        Move::MagicalLeaf => todo!(),
    };
}

fn void() {}
fn tackle(active_moves: Query<(&Move, &Parent)>, parent: Query<&mut Transform>) {
    for (move_id, parent) in active_moves
        .iter()
        .filter(|id| matches!(id.0, Move::Tackle))
    {}
}

pub struct MovePlugin;

impl Plugin for MovePlugin {
    fn build(&self, app: &mut App) {
        for input in enum_iterator::all::<Move>() {
            register_systems(app, input)
        }
    }
}

#[derive(Debug, Component)]
pub struct MoveInfo {
    id: Move,
    data: Handle<MoveData>,
    image: Handle<Image>,
}

#[derive(Resource, AssetCollection)]
pub(crate) struct MoveList {
    #[asset(path = "move_data", collection(typed, mapped))]
    pub data: HashMap<Move, Handle<MoveData>>,

    #[asset(path = "moves", collection(typed, mapped))]
    pub image: HashMap<Move, Handle<Image>>,
}

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct MoveData {
    pub(crate) display_name: String,
    pub(crate) spritesheet: AnimationSpritesheet,
    pub(crate) collider: Option<ColliderShape>,
    #[serde(flatten)]
    #[reflect(ignore)]
    pub(crate) extra_info: HashMap<String, ron::Value>,
}

impl MapKey for Move {
    fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
        short_flight::from_asset_path(path)
    }
}

pub struct SpawnMove {
    move_id: Move,
    parent: Entity,
}

impl Command for SpawnMove {
    fn apply(self, world: &mut World) {
        let move_list = world.resource::<MoveList>();
        let Some(handle) = move_list.data.get(&self.move_id) else {
            log::error!("Could not find move data file for {:?}", self.move_id);
            return;
        };
        let Some(img_handle) = move_list.image.get(&self.move_id) else {
            log::error!("Could not find move texture file for {:?}", self.move_id);
            return;
        };

        let move_data = world.resource::<Assets<MoveData>>().get(handle).unwrap();

        world
            .spawn((
                Name::new(move_data.display_name.clone()),
                self.move_id,
                MoveInfo {
                    id: self.move_id,
                    data: handle.clone(),
                    image: img_handle.clone(),
                },
            ))
            .set_parent(self.parent);
    }
}
