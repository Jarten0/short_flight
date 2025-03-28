use super::NPC;
use crate::assets::AnimationSpritesheet;
use bevy::prelude::*;
use bevy_sprite3d::Sprite3d;
use short_flight::animation::AnimType;

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
#[require(NPC)]
pub(crate) struct NPCAnimation {
    pub current: AnimType,
    /// how far the animation has progressed in seconds.
    /// the name "frame" is a bit archaic in the context, but its familiarity is why I named it as such.
    pub frame: f32,
    /// the direction the npc is facing
    pub direction: Vec3,
    pub spritesheet: AnimationSpritesheet,
}

impl NPCAnimation {
    pub fn new(spritesheet: AnimationSpritesheet) -> NPCAnimation {
        Self {
            current: AnimType::Idle,
            direction: Vec3::NEG_Z,
            spritesheet,
            frame: 0.0,
        }
    }

    pub fn update(&mut self, delta: f32) {
        if self.spritesheet[self.current].process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
        };
    }

    pub fn get_current_atlas(&self) -> Option<TextureAtlas> {
        Some(TextureAtlas {
            layout: self.spritesheet.atlas.as_ref()?.clone_weak(),
            index: self.frame.floor() as usize,
        })
    }
}

pub(super) fn update_sprite_timer(mut npcs: Query<&mut NPCAnimation>, delta: Res<Time>) {
    for mut anim in &mut npcs {
        anim.update(delta.delta_secs());
    }
}

pub(super) fn update_npc_sprites(mut npcs: Query<(&mut Sprite3d, &NPCAnimation)>) {
    for (mut sprite, anim) in &mut npcs {
        sprite.texture_atlas = anim.get_current_atlas();
    }
}
