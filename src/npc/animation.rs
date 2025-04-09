use super::NPC;
use crate::assets::AnimationSpritesheet;
use crate::moves::interfaces::MoveInfo;
use crate::player::{ClientQuery, MarkerUgh, Shaymin};
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_inspector_egui::egui::epaint::text::layout;
use short_flight::animation::{AnimType, AnimationData, AnimationDirLabel};
use short_flight::sprite3d::Sprite3d;

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
#[require(NPC)]
pub(crate) struct NPCAnimation {
    current: AnimType,
    /// how far the animation has progressed in seconds.
    /// the name "frame" is a bit archaic in the context, but its familiarity is why I named it as such.
    frame: f32,
    /// the direction the npc is facing
    direction: Dir2,
    pub animations: HashMap<AnimType, AnimationData>,
    pub spritesheet: AnimationSpritesheet,
    pub loop_: bool,
}

impl NPCAnimation {
    pub fn new(spritesheet: AnimationSpritesheet) -> NPCAnimation {
        Self {
            current: AnimType::Idle,
            direction: Dir2::NEG_Y,
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
                self.start_animation(AnimType::Idle, None);
            }
            true
        } else {
            false
        }
    }

    pub fn get_current_atlas(&self) -> Option<TextureAtlas> {
        let Some(handle) = self.spritesheet.atlas.as_ref() else {
            return (None);
        };
        let layout = handle.clone_weak();

        let Some((index, _)) = self.get_atlas_index() else {
            return (None);
        };

        let index = self.frame.floor() as usize + (index * self.spritesheet.max_frames as usize);

        (Some(TextureAtlas { layout, index }))
    }

    fn get_atlas_index(&self) -> Option<(usize, BVec2)> {
        let mut index = 0;
        for id in &self.spritesheet.animations {
            if self.current == *id {
                break;
            }
            index += self.animations[id]
                .direction_label
                .directional_sprite_count() as usize;

            if Some(id) == self.spritesheet.animations.last() {
                return None;
            }
        }
        let (offset, flip) = self
            .animation_data()
            .direction_label
            .get_index_offset(AnimationDirLabel::cardinal(self.direction));
        index += offset;
        Some((index, flip))
    }

    pub fn frame(&self) -> f32 {
        self.frame
    }

    pub fn current(&self) -> AnimType {
        self.current
    }

    pub fn direction(&self) -> Dir2 {
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

    pub fn update_direction(&mut self, direction: Dir2) {
        self.direction = direction;
    }

    /// Should only be called if [`AnimationData::is_blocking`] is false
    pub fn start_animation(&mut self, animation: AnimType, direction: Option<Dir2>) {
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
        if anim.update(delta.delta_secs() * 4.) {
            for child in children.iter() {
                if move_query.get(*child).is_ok() {
                    commands.entity(*child).despawn();
                }
            }
        }
    }
}

pub(super) fn update_npc_sprites(
    mut npcs: Query<(
        &mut Sprite3d,
        &MeshMaterial3d<StandardMaterial>,
        AnyOf<(&NPCAnimation, &MarkerUgh)>,
    )>,
    client: ClientQuery<Option<&NPCAnimation>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut sprite, material, options) in &mut npcs {
        let (Some(atlas)) = (match options.0.or(*client) {
            Some(anim) => anim.get_current_atlas(),
            None => (None),
        }) else {
            continue;
        };

        sprite.texture_atlas = Some(atlas);

        // custom flip code to try and flip atlased sprites in place instead of as the whole texture
        let Some(anim) = options.0.or(*client) else {
            continue;
        };

        let Some((index, flip)) = anim.get_atlas_index() else {
            continue;
        };

        if flip != sprite.flip {
            if let Some(material) = materials.get_mut(material) {
                sprite.flip = flip;
                // see StandardMaterial::flip for the base version of this
                material.uv_transform = if flip.x {
                    // StandardMaterial::FLIP_HORIZONTAL
                    Affine2 {
                        matrix2: Mat2::from_cols(Vec2::new(-1.0, 0.0), Vec2::Y),
                        // translation: Vec2::X,
                        translation: Vec2::X * (anim.frame().floor() + 1.)
                            / anim.spritesheet.max_frames as f32,
                    }
                } else {
                    Affine2::default()
                } * if flip.y {
                    // StandardMaterial::FLIP_VERTICAL
                    Affine2 {
                        matrix2: Mat2::from_cols(Vec2::X, Vec2::new(0.0, -1.0)),
                        // translation: Vec2::Y,
                        translation: Vec2::Y * (index as f32 + 1.)
                            / anim.spritesheet.total_variants as f32,
                    }
                } else {
                    Affine2::default()
                };
            }
        }
    }
}
