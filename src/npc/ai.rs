use bevy::prelude::*;

use super::NPCInfo;

fn run_enemy_npc_ai(
    mut query: Query<(&NPCInfo, &mut Transform, &GlobalTransform)>,
    query2: Query<(&NPCInfo, &Transform, &GlobalTransform)>,

) {
    for (npc, transform, gt) in &mut query {
        let NPCInfo::Enemy {} = npc else { continue };
        for (npc, transform2, gt2) in &query2 {
            match npc {
                NPCInfo::Team => (),
                _ => continue,
            }

            transform.translation += gt2.translation() - gt.translation(); 
        }
    }
}
