use super::NPC;
use crate::assets::AnimationSpritesheet;
use bevy::prelude::*;
use bevy_sprite3d::Sprite3d;
use short_flight::animation::AnimType;

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
#[require(NPC)]
pub struct NPCAnimation {
    pub current: AnimType,
    /// how far the animation has progressed in seconds.
    /// the name "frame" is a bit archaic in the context, but its familiarity is why I named it as such.
    pub frame: f32,
    /// the direction the npc is facing
    pub direction: Vec2,
    pub spritesheet: AnimationSpritesheet,
}

impl NPCAnimation {
    pub fn update(&mut self, delta: f32) {
        if self.spritesheet[self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }
}

/// should be called on any newly spawned NPC
pub(super) fn setup_texture_atlas(mut anim: &mut NPCAnimation, mut sprite: &mut Sprite3d) {
    let max_items = anim
        .spritesheet
        .data
        .0
        .iter()
        .map(|data| data.1.frames)
        .max()
        .unwrap();
    let texture_layout = TextureAtlasLayout::from_grid(
        UVec2::splat(32),
        max_items,
        anim.spritesheet.animations.len() as u32,
        None,
        None,
    );
}

pub(super) fn update_sprite_timer(mut npcs: Query<&mut NPCAnimation>, delta: Res<Time>) {
    for mut anim in &mut npcs {
        anim.update(delta.delta_secs());
    }
}

pub(super) fn update_npc_sprites(mut npcs: Query<(&mut Sprite3d, &mut NPCAnimation)>) {
    for (sprite, anim) in &mut npcs {
        // sprite.
        // anim.update(delta);
    }
}
