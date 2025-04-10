use super::stats::Damage;
use super::stats::Health;
use super::NPCInfo;
use super::NPC;
use crate::assets::AnimationSpritesheet;
use crate::moves::interfaces::Moves;
use crate::moves::Move;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use serde::Deserialize;
use serde::Serialize;
use short_flight::collision::{BasicCollider, ColliderShape};

#[derive(Resource, AssetCollection)]
pub(crate) struct NPCAlmanac {
    #[asset(path = "npc_data", collection(typed, mapped))]
    pub data_files: HashMap<NPC, Handle<NPCData>>,

    #[asset(path = "npcs", collection(typed, mapped))]
    pub image_files: HashMap<NPC, Handle<Image>>,
}

#[derive(Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct NPCData {
    pub(crate) display_name: String,
    pub(crate) info: NPCInfo,
    pub(crate) spritesheet: AnimationSpritesheet,
    #[serde(default)]
    pub(crate) collider: Option<ColliderShape>,
    #[serde(default)]
    pub(crate) stats: Option<(Health, Damage)>,
    #[serde(default)]
    pub(crate) moves: Option<Moves>,
}

pub(crate) fn validate_npc_data(
    mut npc_datas: ResMut<Assets<NPCData>>,
    asset_server: Res<AssetServer>,
) {
    for (id, data) in npc_datas.iter_mut() {
        data.spritesheet.atlas = Some(asset_server.add(data.spritesheet.get_atlas_layout()));
        assert!(data.spritesheet.atlas.is_some());
    }
}
