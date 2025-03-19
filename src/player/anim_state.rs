use std::collections::HashMap;

use super::assets::ShayminAssets;
use crate::assets::AnimationAssets;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use short_flight::animation::{AnimType, AnimationData};

/// Handles the state managment of the player
#[derive(Debug, Component)]
pub struct ShayminAnimation {
    pub current: AnimType,
    /// how far the animation has progressed in seconds. the name "frame" is a bit archaic in the context,
    /// but its familiarity is why I named it as such.
    pub frame: f32,
    /// the direction the player is facing
    pub direction: Vec2,
    pub pool: HashMap<AnimType, AnimationData>,
    pub materials: HashMap<AnimType, Handle<StandardMaterial>>,
}

impl ShayminAnimation {
    pub fn update(&mut self, delta: f32) {
        if self.pool[&self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }
}

pub fn animation(
    asset_server: &AssetServer,
    assets: &ShayminAssets,
    anim_assets: Res<Assets<AnimationAssets>>,
) -> ShayminAnimation {
    let materials = materials(asset_server);
    let default = Default::default();
    ShayminAnimation {
        current: AnimType::Idle,
        frame: 0.0,
        pool: anim_assets
            .get(&assets.animations)
            .unwrap_or(&default)
            .0
            .clone(),

        direction: Vec2::ZERO,
        materials,
    }
}

pub fn sprite(collection: &ShayminAssets, mut sprite3d_params: Sprite3dParams) -> Sprite3dBundle {
    // let Some(get) = sprite3d_params.images.get(&collection.shaymin) else {
    //     log::error!("Images are not loaded.");
    //     panic!("Fix image loading issues");
    // };

    // let (mut layout, sources, atlas) = TextureAtlasBuilder::default()
    //     .add_texture(Some(collection.shaymin.id()), &get.clone())
    //     .build()
    //     .unwrap();

    // layout.add_texture(URect::from_corners(UVec2::new(0, 0), UVec2::new(32, 32)));

    // let layout = sprite3d_params.atlas_layouts.add(layout);
    // let image = sprite3d_params.images.add(atlas);

    let sprite = Sprite3dBuilder {
        image: collection.shaymin.clone(),
        pixels_per_metre: 32.0,
        pivot: None,
        // pivot: Some(Vec2::new(0.5, 0.0)),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: false,
        emissive: LinearRgba::rgb(0.0, 0.02, 0.0),
    }
    .bundle(&mut sprite3d_params);
    // .bundle_with_atlas(&mut sprite3d_params, TextureAtlas::from(layout));

    sprite
}

fn materials(asset_server: &AssetServer) -> HashMap<AnimType, Handle<StandardMaterial>> {
    let materials = [
        (AnimType::Idle, palettes::basic::GREEN),
        (AnimType::Walking, palettes::basic::LIME),
        (AnimType::Hurt, palettes::basic::RED),
        (AnimType::Down, palettes::basic::MAROON),
        (AnimType::AttackSwipe, palettes::basic::BLUE),
        (AnimType::AttackTackle, palettes::basic::BLACK),
    ]
    .into_iter()
    .map(|item| {
        (
            item.0,
            asset_server.add(StandardMaterial::from_color(item.1)),
        )
    })
    .collect();
    materials
}

pub fn update_materials(
    mut query: Query<(&mut ShayminAnimation, &mut MeshMaterial3d<StandardMaterial>)>,
    delta: Res<Time<Fixed>>,
) {
    // for (mut handler, mut material_handle) in &mut query {
    //     handler.update(delta.delta_secs());

    //     if let Some(handle) = handler.materials.get(&handler.current) {
    //         **material_handle = handle.clone();
    //     };
    // }
}
