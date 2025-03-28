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
pub(crate) struct NPCAlmanac(pub HashMap<NPC, Handle<NPCData>>);

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone)]
pub(crate) struct NPCData {
    pub(crate) display_name: String,
    pub(crate) info: NPCInfo,
    pub(crate) collider: Option<Collider>,
    pub(crate) stats: Option<(Health, Damage)>,
}

pub(crate) fn load_npcs(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handles = enum_iterator::all()
        .map(|npc: NPC| {
            let path = "npcs/".to_string() + npc.variant_name() + ".ron";
            (npc, asset_server.load::<NPCData>(path))
        })
        .collect();

    commands.insert_resource(NPCAlmanac(handles));
}
