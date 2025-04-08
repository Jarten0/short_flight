use bevy::prelude::*;
use short_flight::animation::AnimType;

use crate::moves::interfaces::SpawnMove;
use crate::moves::tackle::Tackle;
use crate::player::Shaymin;

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
#[derive(Debug, Default, Component, Reflect, Clone)]
pub enum NPCDesicion {
    #[default]
    Idle,
    SetAnimation(AnimType),
    Move {
        direction: Vec3,
    },
    Attack {
        direction: Option<Dir2>,
    },
}

impl Default for NPCActions {
    fn default() -> Self {
        Self::Idle {
            explore: Vec3::ZERO,
        }
    }
}

pub(crate) fn run_enemy_npc_ai(
    query: Query<
        (
            Entity,
            Option<(&NPCInfo, &NPCActions, &NPCAnimation)>,
            Option<&Shaymin>,
            &Transform,
            &GlobalTransform,
        ),
        Or<(With<NPCInfo>, With<Shaymin>)>,
    >,
    mut query2: Query<&mut NPCDesicion>,
) {
    for (entity, npc, player, transform, gtransform) in &query {
        let Some((npc, npc_actions, npc_anim)) = npc else {
            continue;
        };

        // blocking animations shouldnt let them do anything anyways, so skip now to save on the extra work
        if npc_anim.animation_data().is_blocking() {
            continue;
        }
        let result = match *npc_actions {
            NPCActions::Offensive { focus }
                if !can_aggro(npc, {
                    let get = query.get(focus).unwrap();
                    match (get.1, get.2) {
                        (Some(npc), None) => npc.0,
                        (None, Some(player)) => &NPCInfo::Team {},
                        (Some(npc), Some(_)) => npc.0,
                        (None, None) => todo!(),
                    }
                }) =>
            {
                NPCDesicion::SetAnimation(AnimType::Idle)
            }
            NPCActions::Offensive { focus } => {
                let (entity, _, _, other_transform, other_gtransform) = query.get(focus).unwrap();

                let distance = Vec3::from((
                    other_gtransform.translation().xz() - gtransform.translation().xz(),
                    0.0,
                ));

                let attack_range = match npc {
                    NPCInfo::Enemy {} => 2.,
                    NPCInfo::Team {} => 2.,
                    _ => 0. as f32,
                };

                if distance.length_squared() <= attack_range.powi(2) {
                    NPCDesicion::Attack {
                        direction: Dir2::new(distance.normalize_or_zero().xy()).ok(),
                    }
                } else {
                    NPCDesicion::Move {
                        direction: distance.normalize_or_zero().xzy(),
                    }
                }
            }
            NPCActions::Defensive => {
                let mut distance = Vec3::ZERO;

                for (_, other_npc, player, _, other_gtransform) in &query {
                    let other = match (other_npc, player) {
                        (Some(npc), None) => npc.0,
                        (None, Some(player)) => &NPCInfo::Team {},
                        (Some(npc), Some(_)) => npc.0,
                        (None, None) => todo!(),
                    };
                    if can_aggro(npc, other) {
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

        let Ok(mut desicion) = query2.get_mut(entity) else {
            // commands.entity(entity).insert(result);
            continue;
        };

        *desicion = result;
    }
}

pub(crate) fn commit_npc_actions(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &NPCInfo,
        &NPCDesicion,
        &mut NPCAnimation,
        &mut Transform,
    )>,
    time: Res<Time>,
) {
    for (entity, info, desicion, mut anim, mut transform) in &mut query {
        match desicion {
            NPCDesicion::Idle => (),
            NPCDesicion::Move { direction } => {
                transform.translation += direction * time.delta_secs();
            }
            NPCDesicion::Attack { direction } => {
                if !anim.animation_data().is_blocking() {
                    anim.start_animation(AnimType::AttackTackle, *direction);
                    commands.queue(SpawnMove {
                        move_: Tackle,
                        parent: entity,
                    })
                }
            }
            NPCDesicion::SetAnimation(anim_type) => {
                anim.start_animation(*anim_type, None);
            }
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
