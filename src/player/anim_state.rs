use super::assets::ShayminAssets;
use crate::assets::AnimationSpritesheet;
use crate::npc::animation::AnimationHandler;
use bevy::prelude::*;
use short_flight::animation::{AnimType, AnimationDirLabel};
use short_flight::sprite3d::Sprite3dBuilder;

pub fn animation(asset_server: &AssetServer, assets: &ShayminAssets) -> impl Bundle {
    AnimationHandler::new(AnimationSpritesheet::new(
        vec![
            (AnimType::Idle.create_data(1, AnimationDirLabel::FullyDirectional)),
            (AnimType::Walking.create_data(2, AnimationDirLabel::FullyDirectional)),
            (AnimType::AttackShoot.create_data(2, AnimationDirLabel::Horizontal)),
        ]
        .into_iter()
        .map(|value| (value.variant, value))
        .collect(),
        UVec2 { x: 32, y: 32 },
        assets.shaymin.clone(),
        asset_server,
    ))
}

pub fn sprite(collection: &ShayminAssets) -> Sprite3dBuilder {
    Sprite3dBuilder {
        image: collection.shaymin.clone(),
        pixels_per_metre: 32.0,
        pivot: None,
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: false,
        emissive: LinearRgba::rgb(0.0, 0.02, 0.0),
    }
}
