use crate::assets::AnimationSpritesheet;

use super::stats::Damage;
use super::stats::Health;
use super::NPCInfo;
use super::NPC;
use bevy::prelude::*;
use bevy::reflect::Enum;
use serde::Deserialize;
use serde::Serialize;
use short_flight::collision::Collider;
use std::collections::HashMap;

#[derive(Resource)]
pub(crate) struct NPCAlmanac(pub HashMap<NPC, (Handle<NPCData>, Handle<Image>)>);

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct NPCData {
    pub(crate) display_name: String,
    pub(crate) info: NPCInfo,
    pub(crate) collider: Option<Collider>,
    pub(crate) stats: Option<(Health, Damage)>,
    pub(crate) spritesheet: AnimationSpritesheet,
}

pub(crate) fn load_npcs(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handles = enum_iterator::all()
        .map(|npc: NPC| {
            let path = "npcs/".to_string() + npc.variant_name();
            let npc_data = asset_server.load::<NPCData>(path.clone() + ".ron");
            let npc_texture = asset_server.load::<Image>(path.clone() + ".png");
            (npc, (npc_data, npc_texture))
        })
        .collect();

    commands.insert_resource(NPCAlmanac(handles));
}
