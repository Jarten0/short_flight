use super::NPC;
use super::stats::FacingDirection;
use crate::animation::{AnimType, AnimationData, AnimationDirLabel};
use crate::assets::AnimationSpritesheet;
use crate::moves::interfaces::MoveInfo;
use crate::shaymin::{ClientQuery, Shaymin, SpriteChildMarker};
use crate::sprite3d::Sprite3d;
use bevy::color::palettes;
use bevy::math::Affine2;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_inspector_egui::egui::epaint::text::layout;
use thiserror::Error;

/// Handles the state managment of the NPC
#[derive(Debug, Component)]
#[require(FacingDirection)]
pub(crate) struct AnimationHandler {
    current: AnimType,
    /// how far the animation has progressed in seconds.
    /// the name "frame" is a bit archaic in the context, but its familiarity is why I named it as such.
    frame: f32,
    /// time modifier to alter how quickly frame increases
    speed: f32,

    pub animations: HashMap<AnimType, AnimationData>,
    pub spritesheet: AnimationSpritesheet,
    pub looping: bool,
}

#[derive(Debug, Error)]
pub(crate) enum AnimationError {
    #[error("The animation data asset was not set for the animation handler.")]
    AnimationDataAssetMissing,
    #[error("The spritesheet asset was not set for the animation handler.")]
    SpritesheetAssetMissing,
    #[error("The spritesheet asset was not set for the animation handler.")]
    SpritesheetMissing,
    #[error("Animation list is empty.")]
    NoAnimationsListed,
    #[error("Animation not found in animation list.")]
    AnimationNotIncluded(AnimType),
    #[error("The listed animation has no corresponding animation data given.")]
    ListedAnimationMissingData(AnimType),
}

impl AnimationHandler {
    pub fn new(spritesheet: AnimationSpritesheet) -> AnimationHandler {
        debug_assert!(
            spritesheet.data.0.contains_key(&AnimType::Idle),
            "Idle animation not found! Fallback behaviour requires an idle animation"
        );
        Self {
            current: AnimType::Idle,
            animations: spritesheet.data.0.clone(),
            spritesheet,
            frame: 0.0,
            looping: false,
            speed: 4.0,
        }
    }

    pub fn update(&mut self, delta: f32) -> bool {
        let Some(animation_data) = self.animations.get(&self.current) else {
            log::error!("Could not find animation data for {:?}", self.current);
            self.start_animation(AnimType::Idle);
            return true;
        };

        if animation_data.process_timer(&mut self.frame, delta * self.speed) {
            if !self.looping {
                self.start_animation(AnimType::Idle);
            }
            true
        } else {
            false
        }
    }

    pub fn get_current_atlas(
        &self,
        direction: &FacingDirection,
    ) -> Result<TextureAtlas, AnimationError> {
        let Some(handle) = self.spritesheet.atlas.as_ref() else {
            return Err(AnimationError::SpritesheetAssetMissing);
        };
        let layout = handle.clone_weak();

        let (index, _flip) = self.get_atlas_index(direction)?;

        let index = self.frame.floor() as usize + (index * self.spritesheet.max_frames as usize);

        Ok(TextureAtlas { layout, index })
    }

    fn get_atlas_index(
        &self,
        direction: &FacingDirection,
    ) -> Result<(usize, BVec2), AnimationError> {
        let mut index = 0;
        for id in &self.spritesheet.animations {
            if self.current == *id {
                break;
            }
            index += self.animations[id]
                .direction_label
                .directional_sprite_count() as usize;

            if Some(id) == self.spritesheet.animations.last() {
                return Err(AnimationError::AnimationNotIncluded(self.current));
            }
        }
        if self.spritesheet.animations.len() == 0 {
            return Err(AnimationError::NoAnimationsListed);
        }
        let (offset, flip) = self
            .animation_data()
            .ok_or(AnimationError::ListedAnimationMissingData(self.current))?
            .direction_label
            .get_index_offset(direction);
        index += offset;
        Ok((index, flip))
    }

    pub fn frame(&self) -> f32 {
        self.frame * self.speed
    }

    pub fn time(&self) -> f32 {
        self.frame
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn current(&self) -> AnimType {
        self.current
    }

    pub fn animation_data(&self) -> Option<&AnimationData> {
        self.spritesheet.data.0.get(&self.current)
    }

    pub fn get_animation_data(&self) -> Option<&AnimationData> {
        self.spritesheet.data.0.get(&self.current)
    }

    /// Should only be called if [`AnimationData::is_blocking`] is false
    pub fn start_animation(&mut self, animation: AnimType) {
        self.looping = false;
        self.frame = 0.0;
        self.current = animation;
    }
}

pub(super) fn update_anim_handler_timer(
    mut commands: Commands,
    mut npcs: Query<(&mut AnimationHandler, &Children)>,
    move_query: Query<&MoveInfo>,
    delta: Res<Time>,
) {
    for (mut anim, children) in &mut npcs {
        if anim.update(delta.delta_secs()) {
            for child in children.iter() {
                if move_query.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }
    }
}

pub(super) fn update_anim_sprites(
    mut npcs: Query<
        (
            Entity,
            &mut Sprite3d,
            &MeshMaterial3d<StandardMaterial>,
            &AnimationHandler,
            &FacingDirection,
        ),
        Without<SpriteChildMarker>,
    >,
    mut client_child: Query<
        (Entity, &mut Sprite3d, &MeshMaterial3d<StandardMaterial>),
        With<SpriteChildMarker>,
    >,
    client: ClientQuery<(&AnimationHandler, &FacingDirection)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    kb: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,
    transform_query: Query<&GlobalTransform>,
    mesh_query: Query<&Mesh3d>,
    meshes: ResMut<Assets<Mesh>>,
) {
    let draw_gizmos = kb.pressed(KeyCode::KeyF);

    let add_client_data = |(e, s, m)| {
        return (e, s, m, client.0, client.1);
    };

    let npcs = npcs
        .iter_mut()
        .chain(client_child.iter_mut().map(add_client_data));

    for (entity, mut sprite, material, anim, dir) in npcs {
        if draw_gizmos && let Ok(transform) = transform_query.get(entity) {
            let translation = transform.translation();
            gizmos.rect(
                Isometry3d::new(
                    translation.with_y(translation.y + 2.),
                    Quat::from_rotation_x(f32::to_radians(-90.0)),
                ),
                Vec2::ONE,
                palettes::basic::WHITE,
            );

            // if let Ok(_mesh3d) = mesh_query.get(entity)
            //     && let Some(mesh) = meshes.get(&_mesh3d.0)
            // {
            //     for vertex in mesh
            //         .attribute(Mesh::ATTRIBUTE_POSITION)
            //         .unwrap()
            //         .as_float3()
            //         .unwrap()
            //     {
            //         gizmos.sphere(
            //             Vec3::from(*vertex) + translation,
            //             0.1,
            //             palettes::basic::YELLOW,
            //         );
            //     }
            // }
        }
        let Ok(atlas) = anim.get_current_atlas(dir).inspect_err(|err| {
            log::error!("Could not get animation sprite for {}! [{}]", entity, err)
        }) else {
            continue;
        };

        sprite.texture_atlas = Some(atlas);

        // custom flip code to try and flip atlased sprites in place instead of as the whole texture
        let Ok((_index, flip)) = anim.get_atlas_index(dir).inspect_err(|err| {
            log::error!(
                "Could not get atlas index for sprite flipping {}! [{}]",
                entity,
                err
            )
        }) else {
            continue;
        };

        // if flip != sprite.flip {
        if let Some(material) = materials.get_mut(material) {
            sprite.flip = flip;
            // see StandardMaterial::flip for the base version of this
            material.uv_transform = if flip.x {
                // StandardMaterial::FLIP_HORIZONTAL
                let columns = anim.spritesheet.max_frames as f32;
                let column = (anim.time().floor() * 2.) + 1.;
                let offset = Vec2::X * f32::clamp(column / columns, 1. / columns, columns * 2.);
                Affine2 {
                    matrix2: Mat2::from_cols(Vec2::NEG_X, Vec2::Y),
                    // translation: Vec2::X,
                    translation: offset,
                }
            } else {
                Affine2::IDENTITY
            };

            if draw_gizmos && let Ok(transform) = transform_query.get(entity) {
                gizmos.rect(
                    Isometry3d::new(
                        transform.translation()
                            + material.uv_transform.translation.extend(2.1).xzy(),
                        Quat::from_rotation_x(f32::to_radians(-90.0)),
                    ),
                    Vec2::ONE / 2.,
                    palettes::basic::FUCHSIA,
                );
            }
        }
    }
}
