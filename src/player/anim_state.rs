use bevy::color::palettes;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use short_flight::animation::{AnimType, AnimationData};

/// Handles the state managment of the player
#[derive(Debug, Reflect, Component)]
pub struct ShayminAnimation {
    pub current: AnimType,
    /// how far the animation has progressed in seconds. the name "frame" is a bit archaic in the context,
    /// but its familiarity is why I named it as such.
    pub frame: f32,
    pub pool: HashMap<AnimType, AnimationData>,
    pub materials: HashMap<AnimType, Handle<StandardMaterial>>,
}

impl ShayminAnimation {
    const EXPECT: &str = "Expected to find animation data at assets/animation/shaymin.ron";
    const PATH: &str = "assets/animation/shaymin.ron";

    pub fn new(asset_server: &mut AssetServer) -> Self {
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
        Self {
            current: AnimType::Idle,
            frame: 0.0,
            pool: Self::animations().expect(Self::EXPECT),
            materials,
        }
    }

    pub fn animations() -> Option<HashMap<AnimType, AnimationData>> {
        short_flight::deserialize_file(Self::PATH).ok()
    }

    pub fn update(&mut self, delta: f32) {
        if self.pool[&self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }
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
