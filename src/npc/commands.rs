use crate::npc::ai::{NPCActions, NPCDesicion};
use crate::player::Shaymin;

use super::animation::NPCAnimation;
use super::{animation, file::NPCAlmanac, file::NPCData, NPCInfo, NPC};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use short_flight::collision::{BasicCollider, ColliderShape, CollisionLayers};
use short_flight::sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dParams};

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
        let stats = (
            data.stats.clone().unwrap(),
            NPCAnimation::new(data.spritesheet.clone()),
            NPCActions::Offensive {
                focus: world
                    .query_filtered::<Entity, With<Shaymin>>()
                    .single(&world),
            },
            NPCDesicion::default(),
        );
        let collider_shape = data.collider.clone();
        let atlas = TextureAtlas {
            layout: data.spritesheet.atlas.clone().unwrap(),
            index: 0,
        };
        let npcinfo = data.info.clone();
        let moves = data.moves.clone();

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

        let mut entity = world.spawn((required, sprite_3d_bundle));

        if let Some(shape) = collider_shape {
            entity.insert(BasicCollider::new(
                true,
                shape,
                CollisionLayers::NPC,
                CollisionLayers::Wall | CollisionLayers::NPC | CollisionLayers::Projectile,
            ));
        }

        if let Some(moves) = moves {
            entity.insert(moves);
        }

        match npcinfo {
            NPCInfo::None => (),
            NPCInfo::Silent => (),
            NPCInfo::Enemy { .. } => {
                entity.insert(stats);
            }
            NPCInfo::Team { .. } => {
                entity.insert(stats);
            }
        };

        let id = entity.id();

        log::info!(
            "Spawned NPC: {}",
            world.query::<&Name>().get(world, id).unwrap().as_str()
        );
    }
}
