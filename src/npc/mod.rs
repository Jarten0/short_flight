use crate::assets::AssetStates;
use bevy::prelude::*;
use bevy::reflect::Enum;
use bevy_asset_loader::prelude::*;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use short_flight::animation::{AnimType, AnimationData};
use std::collections::HashMap;
pub mod ai;
pub mod stats;

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (stats::query_dead))
            .add_systems(Startup, load_npcs);
    }
}

/// Marks this entity as an in-world NPC, that can interact with the player and the world via
/// collision, player interact, contact damage,
/// and can perform actions via NPC AI.
#[derive(Component, Default, Reflect, Clone, PartialEq, Eq, PartialOrd, Sequence, Hash)]
#[require(NPCInfo)]
pub enum NPC {
    /// npc missing identifier
    #[default]
    Void,
    Geodude,
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

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
pub struct NPCAnimation {
    pub current: AnimType,
    /// how far the animation has progressed in seconds. the name "frame" is a bit archaic in the context,
    /// but its familiarity is why I named it as such.
    pub frame: f32,
    /// the direction the npc is facing
    pub direction: Vec2,
    pub pool: HashMap<AnimType, AnimationData>,
}

impl NPCAnimation {
    pub fn update(&mut self, delta: f32) {
        if self.pool[&self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }
}

#[derive(Resource)]
pub struct NPCAlmanac(pub HashMap<NPC, Handle<NPCData>>);

#[derive(Debug, Asset, Reflect, Serialize, Deserialize)]
pub struct NPCData {
    display_name: String,
    info: NPCInfo,
}

fn load_npcs(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handles = enum_iterator::all::<NPC>().map(|npc| {
        let path = "npcs/".to_string() + npc.variant_name();
        (npc, asset_server.load::<NPCData>(path))
    });

    commands.insert_resource(NPCAlmanac(handles.collect()));
}
