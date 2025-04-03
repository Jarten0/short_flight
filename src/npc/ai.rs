use bevy::prelude::*;

use super::animation::NPCAnimation;
use super::NPCInfo;

/// Describes the various states an NPC can be in,
/// which influences how their AI makes decisions
/// as for what to do at any given moment.
#[derive(Debug, Component, Clone, Reflect)]
pub enum NPCActions {
    /// Will try to maneuver themselves so as to give the player little room to breathe.
    Offensive { focus: Entity },
    /// Will try to maneuver so as to get some decent space from
    Defensive,
    /// Will do nada. If `explore` is not zero, then pick a direction and follow it as best as can be.
    Idle { explore: Vec3 },
}

/// What the enemy will choose to do in any given frame
pub enum NPCDesicion {
    Idle,
    Move { direction: Vec3 },
    Attack { direction: Vec2 },
}

impl Default for NPCActions {
    fn default() -> Self {
        Self::Idle {
            explore: Vec3::ZERO,
        }
    }
}

pub(crate) fn run_enemy_npc_ai(
    mut query: Query<(
        Entity,
        &NPCInfo,
        &mut NPCActions,
        &mut NPCAnimation,
        &mut Transform,
        &GlobalTransform,
    )>,
    query2: Query<(Entity, &NPCInfo, &Transform, &GlobalTransform)>,
    time: Res<Time>,
) {
    for (entity, npc, mut npc_actions, mut npc_anim, mut transform, gtransform) in &mut query {
        let result = match *npc_actions {
            NPCActions::Offensive { focus } => {
                let (entity, other_npc, other_transform, other_gtransform) =
                    query2.get(focus).unwrap();

                if !can_aggro(npc, other_npc) {
                    *npc_actions = NPCActions::default();
                    continue;
                }

                let distance = Vec3::from((
                    gtransform.translation().xz() - other_gtransform.translation().xz(),
                    0.0,
                ));

                let attack_range = match npc {
                    _ => 0. as f32,
                    NPCInfo::Enemy {} => todo!(),
                    NPCInfo::Team {} => todo!(),
                };

                if distance.length_squared() <= attack_range.powi(2) {
                    NPCDesicion::Attack {
                        direction: distance.normalize_or_zero().xy() * time.delta_secs(),
                    }
                } else {
                    NPCDesicion::Move {
                        direction: distance.normalize_or_zero().xzy() * time.delta_secs(),
                    }
                }
            }
            NPCActions::Defensive => {
                let mut distance = Vec3::ZERO;

                for (entity, other_npc, other_transform, other_gtransform) in &query2 {
                    if can_aggro(npc, other_npc) {
                        distance += gtransform.translation() - other_gtransform.translation();
                    }
                }

                distance = distance.normalize_or_zero();

                NPCDesicion::Move {
                    direction: distance,
                }
            }
            NPCActions::Idle { explore } => {
                if explore != Vec3::ZERO {
                    NPCDesicion::Move { direction: explore }
                } else {
                    NPCDesicion::Idle
                }
            }
        };

        match result {
            NPCDesicion::Idle => (),
            NPCDesicion::Move { direction } => {
                transform.translation += direction * time.delta_secs();
            }
            NPCDesicion::Attack { direction } => todo!(),
        }
    }
}

fn can_aggro(npc: &NPCInfo, other_npc: &NPCInfo) -> bool {
    if matches!(npc, NPCInfo::Enemy { .. }) {
        matches!(other_npc, NPCInfo::Team { .. })
    } else if matches!(npc, NPCInfo::Team { .. }) {
        matches!(other_npc, NPCInfo::Enemy { .. })
    } else {
        false
    }
}
