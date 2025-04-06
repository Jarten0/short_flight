use super::NPC;
use crate::assets::AnimationSpritesheet;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_sprite3d::Sprite3d;
use short_flight::animation::{AnimType, AnimationData};

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
#[require(NPC)]
pub(crate) struct NPCAnimation {
    current: AnimType,
    /// how far the animation has progressed in seconds.
    /// the name "frame" is a bit archaic in the context, but its familiarity is why I named it as such.
    frame: f32,
    /// the direction the npc is facing
    direction: Vec3,
    pub animations: HashMap<AnimType, AnimationData>,
    pub spritesheet: AnimationSpritesheet,
}

impl NPCAnimation {
    pub fn new(spritesheet: AnimationSpritesheet) -> NPCAnimation {
        Self {
            current: AnimType::Idle,
            direction: Vec3::NEG_Z,
            animations: spritesheet.data.0.clone(),
            spritesheet,
            frame: 0.0,
        }
    }

    pub fn update(&mut self, delta: f32) -> bool {
        let Some(animation_data) = self.animations.get(&self.current) else {
            log::error!("Could not find animation data for {:?}", self.current);
            self.frame += delta;
            // self.start_animation(AnimType::Idle, None);
            return true;
        };

        if animation_data.process_timer(&mut self.frame, delta) {
            self.current = AnimType::Idle;
            true
        } else {
            false
        }
    }

    pub fn get_current_atlas(&self) -> Option<TextureAtlas> {
        Some(TextureAtlas {
            layout: self.spritesheet.atlas.as_ref()?.clone_weak(),
            index: self.frame.floor() as usize
                + (self
                    .spritesheet
                    .animations
                    .iter()
                    .enumerate()
                    .find(|value| *value.1 == self.current)
                    .map(|value| value.0)
                    .unwrap_or(0)
                    * self.spritesheet.max_items as usize),
        })
    }

    pub fn frame(&self) -> f32 {
        self.frame
    }

    pub fn current(&self) -> AnimType {
        self.current
    }

    pub fn direction(&self) -> Vec3 {
        self.direction
    }

    pub fn animation_data(&self) -> &AnimationData {
        self.spritesheet
            .data
            .0
            .get(&self.current)
            .unwrap_or_else(|| {
                log::error!(
                    "Could not find animation data for {:?} in {:#?}",
                    self.current,
                    self.animations
                );
                panic!("Failed to find animation data")
            })
    }

    pub fn get_animation_data(&self) -> Option<&AnimationData> {
        self.spritesheet.data.0.get(&self.current)
    }

    pub fn start_animation(&mut self, animation: AnimType, direction: Option<Vec3>) {
        self.frame = 0.0;
        self.current = animation;
        if let Some(direction) = direction {
            self.direction = direction;
        }
    }
}

pub(super) fn update_sprite_timer(
    mut commands: Commands,
    mut npcs: Query<(Entity, &mut NPCAnimation)>,
    delta: Res<Time>,
) {
    for (parent, mut anim) in &mut npcs {
        if anim.update(delta.delta_secs() * 3.) {
            commands.entity(parent).despawn_descendants();
        }
    }
}

pub(super) fn update_npc_sprites(mut npcs: Query<(&mut Sprite3d, &NPCAnimation)>) {
    for (mut sprite, anim) in &mut npcs {
        sprite.texture_atlas = anim.get_current_atlas();
    }
}
