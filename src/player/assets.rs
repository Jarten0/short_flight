use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use serde::{Deserialize, Serialize};
use crate::animation::{AnimType, AnimationData};
use std::collections::HashMap;

use crate::assets::AnimationAssets;

#[derive(AssetCollection, Resource)]
pub struct ShayminAssets {
    #[asset(path = "shaymin/shaymin.png")]
    pub shaymin: Handle<Image>,
    #[asset(path = "shaymin/animations.anim.ron")]
    pub animations: Handle<AnimationAssets>,
}
