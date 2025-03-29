use super::animation::NPCAnimation;
use super::{animation, file::NPCAlmanac, file::NPCData, NPCInfo, NPC};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dParams};

pub struct SpawnNPC {
    pub npc_id: NPC,
    pub position: Vec3,
    pub name: Option<String>,
}

impl Command for SpawnNPC {
    fn apply(self, world: &mut World) {
        let npc_almanac = world.resource::<NPCAlmanac>();
        let npc_data = world.resource::<Assets<NPCData>>();

        let (data_handle, image_handle) = &npc_almanac.0.get(&self.npc_id).unwrap_or_else(|| {
            panic!(
                "Could not find NPC almanac entry for {}",
                self.npc_id as isize
            );
        });
        let image_handle = image_handle.clone();

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
            Transform::from_translation(self.position),
            Name::new(self.name.clone().unwrap_or(data.display_name.clone())),
        );
        let stats = (
            data.stats.clone().unwrap(),
            NPCAnimation::new(data.spritesheet.clone()),
        );
        let collider = data.collider.clone();
        let atlas = TextureAtlas {
            layout: data.spritesheet.atlas.clone().unwrap(),
            index: 0,
        };
        let npcinfo = data.info.clone();

        // Construct a `SystemState` struct, passing in a tuple of `SystemParam`
        // as if you were writing an ordinary system.
        let mut system_state: SystemState<Sprite3dParams> = SystemState::new(world);

        let sprite_3d_bundle = Sprite3dBuilder {
            image: image_handle,
            pixels_per_metre: 32.0,
            pivot: None,
            alpha_mode: AlphaMode::default(),
            unlit: false,
            double_sided: true,
            emissive: LinearRgba::BLACK,
        }
        .bundle_with_atlas(&mut system_state.get_mut(world), atlas);

        drop(data);

        let mut entity = world.spawn((required, sprite_3d_bundle));

        if let Some(collider) = collider {
            entity.insert(collider);
        }

        match npcinfo {
            NPCInfo::None => (),
            NPCInfo::Silent => (),
            NPCInfo::Enemy {} => {
                entity.insert(stats);
            }
            NPCInfo::Team => {
                entity.insert(stats);
            }
        };

        log::info!("Spawned NPC: {}", self.name.unwrap_or(data.display_name.clone())

    }
}
