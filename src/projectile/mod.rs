use crate::assets::AnimationSpritesheet;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::mapped::MapKey;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use short_flight::collision::ColliderShape;

#[derive(Resource, AssetCollection)]
pub(crate) struct ProjectileCatalog {
    #[asset(path = "projectile_data", collection(typed, mapped))]
    pub data_files: HashMap<Projectile, Handle<ProjectileData>>,

    #[asset(path = "projectiles", collection(typed, mapped))]
    pub image_files: HashMap<Projectile, Handle<Image>>,
}

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct ProjectileData {
    pub(crate) display_name: String,
    pub(crate) spritesheet: AnimationSpritesheet,
    pub(crate) collider: ColliderShape,
}

/// Marks this entity as a move, aka an attack, that temporarily exists in the world.
#[derive(
    Component, Default, Reflect, Sequence, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Hash,
)]
pub enum Projectile {
    #[default]
    Void = 0,
}
impl MapKey for Projectile {
    fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
        short_flight::from_asset_path(path)
    }
}
