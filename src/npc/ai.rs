use bevy::prelude::*;

use super::NPCInfo;

fn run_enemy_npc_ai(mut query: Query<(&NPCInfo, &mut Transform)>) {
    for (npc, transform) in &mut query {
        let NPCInfo::Enemy {} = npc else { continue };

        // transform.
    }
}
