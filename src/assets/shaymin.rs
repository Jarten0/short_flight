use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use short_flight::animation::AnimationData;

#[derive(AssetCollection, Resource)]
pub struct SpritesCollection {
    #[asset(path = "shaymin/shaymin.png")]
    pub shaymin: Handle<Image>,
    #[asset(path = "shaymin/animations.ron")]
    pub animations: Handle<AnimationData>,
}
