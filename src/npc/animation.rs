use super::NPC;
use crate::assets::AnimationSpritesheet;
use crate::moves::interfaces::MoveInfo;
use crate::player::{ClientQuery, MarkerUgh, Shaymin};
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
    pub loop_: bool,
}

impl NPCAnimation {
    pub fn new(spritesheet: AnimationSpritesheet) -> NPCAnimation {
        Self {
            current: AnimType::Idle,
            direction: Vec3::NEG_Z,
            animations: spritesheet.data.0.clone(),
            spritesheet,
            frame: 0.0,
            loop_: false,
        }
    }

    pub fn update(&mut self, delta: f32) -> bool {
        let Some(animation_data) = self.animations.get(&self.current) else {
            log::error!("Could not find animation data for {:?}", self.current);
            self.frame += delta;
            return true;
        };

        if animation_data.process_timer(&mut self.frame, delta) {
            if !self.loop_ {
                self.start_animation(AnimType::Idle, Some(self.direction));
            }
            true
        } else {
            false
        }
    }

    pub fn get_current_atlas(&self) -> Option<TextureAtlas> {
        let layout = self.spritesheet.atlas.as_ref()?.clone_weak();
        let animation_index = self
            .spritesheet
            .animations
            .iter()
            .enumerate()
            .find_map(|value| (*value.1 == self.current).then_some(value.0))
            .unwrap_or(0);
        let index =
            self.frame.floor() as usize + (animation_index * self.spritesheet.max_items as usize);

        if self.spritesheet.max_items == 2 {
            log::info!("{:?}", (index, animation_index));
        }
        Some(TextureAtlas { layout, index })
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
        self.loop_ = false;
        self.frame = 0.0;
        self.current = animation;
        if let Some(direction) = direction {
            self.direction = direction;
        }
    }
}

pub(super) fn update_sprite_timer(
    mut commands: Commands,
    mut npcs: Query<(&mut NPCAnimation, &Children)>,
    move_query: Query<&MoveInfo>,
    delta: Res<Time>,
) {
    for (mut anim, children) in &mut npcs {
        if anim.update(delta.delta_secs() * 3.) {
            for child in children.iter() {
                if move_query.get(*child).is_ok() {
                    commands.entity(*child).despawn();
                }
            }
        }
    }
}

pub(super) fn update_npc_sprites(
    mut npcs: Query<(&mut Sprite3d, AnyOf<(&NPCAnimation, &MarkerUgh)>)>,
    client: Single<Option<&NPCAnimation>, (With<Shaymin>, ())>,
) {
    for (mut sprite, options) in &mut npcs {
        match options {
            (Some(anim), Some(_)) => {
                sprite.texture_atlas = anim.get_current_atlas();
            }
            (Some(anim), None) => {
                sprite.texture_atlas = anim.get_current_atlas();
            }
            (None, Some(_)) => {
                if let Some(a) = *client {
                    sprite.texture_atlas = a.get_current_atlas();
                };
            }
            _ => (),
        }
    }
}
