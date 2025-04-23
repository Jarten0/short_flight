use super::animation::AnimationHandler;
use super::{NPC, NPCInfo, animation, file::NPCAlmanac, file::NPCData};
use crate::collision::{
    self, BasicCollider, ColliderShape, CollisionLayers, DynamicCollision, ZHitbox,
};
use crate::npc::ai::{NPCActions, NPCDesicion};
use crate::npc::stats::FacingDirection;
use crate::player::{self, Shaymin};
use crate::sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dParams};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;

/// Spawns an NPC with the given NPC asset data
///
/// Will panic if run before the assets can be loaded during [`PreStartup`] in [`crate::npc::file::load_npcs`]
pub struct SpawnNPC {
    pub npc_id: NPC,
    pub position: Vec3,
    pub name: Option<String>,
}

impl Command for SpawnNPC {
    fn apply(self, world: &mut World) {
        let npc_almanac = world.resource::<NPCAlmanac>();
        let npc_data = world.resource::<Assets<NPCData>>();

        let data_handle = npc_almanac.data_files.get(&self.npc_id).unwrap_or_else(|| {
            panic!(
                "Could not find NPC almanac entry for {}",
                self.npc_id as isize
            );
        });
        let image_handle = npc_almanac
            .image_files
            .get(&self.npc_id)
            .unwrap_or_else(|| {
                panic!(
                    "Could not find NPC almanac entry for {}",
                    self.npc_id as isize
                );
            })
            .clone();

        let data = npc_data
            .get(data_handle)
            .unwrap_or_else(|| {
                panic!(
                    "Could not find NPC data asset for entry {}",
                    self.npc_id as isize
                );
            })
            .clone();

        let required = (
            self.npc_id,
            data.info.clone(),
            Transform::from_translation(self.position + Vec3::new(0., 0.2, 0.))
                .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0))),
            Name::new(self.name.clone().unwrap_or(data.display_name.clone())),
        );
        let animation = (
            AnimationHandler::new(data.spritesheet.clone()),
            NPCActions::Offensive {
                focus: world
                    .query_filtered::<Entity, With<Shaymin>>()
                    .single(&world)
                    .unwrap(),
            },
            NPCDesicion::default(),
            FacingDirection::default(),
        );
        let collider_shape = data.collider.clone();
        let atlas = TextureAtlas {
            layout: data.spritesheet.atlas.clone().unwrap(),
            index: 0,
        };
        let npcinfo = data.info.clone();
        let moves = data.moves.clone();
        let stats = data.stats.clone();

        // Construct a `SystemState` struct, passing in a tuple of `SystemParam`
        // as if you were writing an ordinary system.
        let mut system_state: SystemState<Sprite3dParams> = SystemState::new(world);

        let sprite_3d_bundle = Sprite3dBuilder {
            image: image_handle,
            pixels_per_metre: 32.0,
            pivot: None,
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: false,
            double_sided: true,
            emissive: LinearRgba::BLACK,
        }
        .bundle_with_atlas(&mut system_state.get_mut(world), atlas);

        system_state.apply(world);

        drop(data);

        let name = required.3.to_string();
        let mut entity = world.spawn((required, sprite_3d_bundle));
        let mut observe_collision = false;
        let mut has_stats = false;

        if let Some(shape) = collider_shape {
            entity.insert((
                BasicCollider::new(
                    true,
                    shape,
                    CollisionLayers::NPC,
                    CollisionLayers::Wall | CollisionLayers::NPC | CollisionLayers::Projectile,
                ),
                ZHitbox::default(),
                DynamicCollision::default(),
            ));
            observe_collision = true;
        }

        if let Some(moves) = moves {
            entity.insert(moves);
        }

        match (npcinfo, stats) {
            (NPCInfo::None, _) => (),
            (NPCInfo::Silent, _) => {
                entity.insert(animation);
            }
            (NPCInfo::Enemy { .. }, Some(stats)) | (NPCInfo::Team { .. }, Some(stats)) => {
                entity.insert(animation);
                entity.insert(stats);
                has_stats = true;
            }
            _ => panic!("Invalid NPC configuration (Missing stats in {:#?})", name),
        };

        // these are called later because calling entity.insert() after .observe() seemingly crashes things
        if observe_collision {
            entity.observe(collision::physics::move_out_from_tilemaps);
        }
        if has_stats {
            entity.observe(collision::physics::take_hits);
        }

        log::info!("Spawned NPC: {}", name);
    }
}
