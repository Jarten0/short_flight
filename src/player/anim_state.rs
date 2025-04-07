use super::assets::ShayminAssets;
use crate::assets::AnimationSpritesheet;
use crate::npc::animation::NPCAnimation;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use short_flight::animation::AnimType;

pub fn animation(asset_server: &AssetServer, assets: &ShayminAssets) -> impl Bundle {
    NPCAnimation::new(AnimationSpritesheet::new(
        vec![
            (AnimType::Idle.create_data(2)),
            (AnimType::WalkingRight.create_data(2)),
            (AnimType::Walking.create_data(2)),
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
