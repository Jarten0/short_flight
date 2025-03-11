use crate::assets::shaymin::SpritesCollection;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
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
    const EXPECT: &str = "Expected to find animation data at assets/animation/shaymin.ron";
    const PATH: &str = "assets/animation/shaymin.ron";

    pub fn animations() -> Option<HashMap<AnimType, AnimationData>> {
        short_flight::deserialize_file(Self::PATH).ok()
    }

    pub fn update(&mut self, delta: f32) {
        if self.pool[&self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }
}

pub fn animation(asset_server: &AssetServer) -> ShayminAnimation {
    let materials = materials(asset_server);

    ShayminAnimation {
        current: AnimType::Idle,
        frame: 0.0,
        pool: ShayminAnimation::animations().expect(ShayminAnimation::EXPECT),
        direction: Vec2::ZERO,
        materials,
    }
}

pub fn sprite(
    asset_server: &AssetServer,
    collection: Res<SpritesCollection>,
    mut sprite3d_params: Sprite3dParams,
) -> Sprite3dBundle {
    let Some(get) = sprite3d_params.images.get(&collection.shaymin) else {
        log::error!("Images are not loaded.");
        panic!("Fix image loading issues");
    };

    let (layout, sources, atlas) = TextureAtlasBuilder::default()
        .add_texture(Some(collection.shaymin.id()), &get.clone())
        .build()
        .unwrap();

    let layout = asset_server.add(layout);
    let atlas = asset_server.add(atlas);

    let sprite = Sprite3dBuilder::default()
        .bundle_with_atlas(&mut sprite3d_params, TextureAtlas::from(layout));
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
    for (mut handler, mut material_handle) in &mut query {
        handler.update(delta.delta_secs());

        if let Some(handle) = handler.materials.get(&handler.current) {
            **material_handle = handle.clone();
        };
    }
}
