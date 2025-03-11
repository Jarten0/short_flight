use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use serde::{Deserialize, Serialize};
use short_flight::animation::{AnimType, AnimationData};
use std::collections::HashMap;

#[derive(AssetCollection, Resource)]
pub struct ShayminAssets {
    #[asset(path = "shaymin/shaymin.png")]
    pub shaymin: Handle<Image>,
    #[asset(path = "shaymin/animations.ron")]
    pub animations: Handle<AnimationAsset>,
}

#[derive(Debug, Default, Asset, Reflect, Deserialize, Serialize)]
pub(super) struct AnimationAsset(pub HashMap<AnimType, AnimationData>);
