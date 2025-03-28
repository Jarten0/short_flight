use super::{animation, file::NPCAlmanac, file::NPCData, NPCInfo, NPC};
use bevy::prelude::*;

pub struct SpawnNPC {
    pub npc_id: NPC,
    pub position: Vec3,
}

impl Command for SpawnNPC {
    fn apply(self, world: &mut World) {
        let npc_almanac = world.resource::<NPCAlmanac>();
        let npc_data = world.resource::<Assets<NPCData>>();

        let data = npc_data
            .get(npc_almanac.0.get(&self.npc_id).unwrap_or_else(|| {
                panic!(
                    "Could not find NPC almanac entry for {}",
                    self.npc_id as isize
                );
            }))
            .unwrap_or_else(|| {
                panic!(
                    "Could not find NPC data asset for entry {}",
                    self.npc_id as isize
                );
            })
            .clone();

        let mut entity = world.spawn((
            self.npc_id,
            data.info.clone(),
            Transform::from_translation(self.position),
            Name::new(data.display_name.clone()),
        ));

        if let Some(collider) = data.collider {
            entity.insert(collider);
        }

        match data.info {
            NPCInfo::None => (),
            NPCInfo::Silent => (),
            NPCInfo::Enemy {} => {
                let stats = data.stats.unwrap();
                entity.insert((stats.0, stats.1));
            }
            NPCInfo::Team => {
                let stats = data.stats.unwrap();
                entity.insert((stats.0, stats.1));
            }
        }
    }
}
