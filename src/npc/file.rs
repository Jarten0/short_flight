use super::NPC;
use super::NPCInfo;
use super::stats::Damage;
use super::stats::Health;
use crate::assets::AnimationSpritesheet;
use crate::collision::{BasicCollider, ColliderShape};
use crate::moves::Move;
use crate::moves::interfaces::Moves;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use serde::Deserialize;
use serde::Serialize;

#[derive(Resource, AssetCollection)]
pub(crate) struct NPCAlmanac {
    #[asset(path = "npc_data", collection(typed, mapped))]
    pub data_files: HashMap<NPC, Handle<NPCData>>,

    #[asset(path = "npcs", collection(typed, mapped))]
    pub image_files: HashMap<NPC, Handle<Image>>,
}

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
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
    mut asset_events: EventReader<AssetEvent<NPCData>>,
    mut npc_datas: ResMut<Assets<NPCData>>,
    asset_server: Res<AssetServer>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } => {
                let data = npc_datas.get_mut(*id).unwrap();
                data.spritesheet.atlas =
                    Some(asset_server.add(data.spritesheet.get_atlas_layout()));
            }
            AssetEvent::LoadedWithDependencies { id } => {
                let data = npc_datas.get_mut(*id).unwrap();
                data.spritesheet.atlas =
                    Some(asset_server.add(data.spritesheet.get_atlas_layout()));
            }
            _ => (),
        }
    }

    for (id, data) in npc_datas
        .iter_mut()
        .inspect(|item| log::info!("{:?}", item.1))
    {
        if asset_server.is_loaded(id) {
            if !(data.spritesheet.atlas.is_some()) {
                data.spritesheet.atlas =
                    Some(asset_server.add(data.spritesheet.get_atlas_layout()));
            }
        }
    }
}
