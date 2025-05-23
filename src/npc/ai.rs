use crate::animation::AnimType;
use bevy::prelude::*;

use crate::moves::Move;
use crate::moves::interfaces::{MoveData, MoveInterfaces, MoveList, Moves, SpawnMove};
use crate::moves::tackle::Tackle;
use crate::player::Shaymin;

use super::NPCInfo;
use super::animation::AnimationHandler;
use super::stats::FacingDirection;

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
    /// Will not try to do anything extra, though may not be completely idle if in the middle of an animation.
    #[default]
    Idle,
    SetAnimation(AnimType),
    /// Will attempt to move in the direction of target.
    /// The target position is relative to the current transform.
    Move {
        target: Vec3,
    },
    /// Will use a move to try and attack the player.
    /// The ambition is as simple as the move.
    BasicAttack {
        direction: Option<Dir2>,
        move_id: Move,
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
            Option<(&NPCInfo, &NPCActions, &AnimationHandler)>,
            Option<&Shaymin>,
            &GlobalTransform,
        ),
        Or<(With<NPCInfo>, With<Shaymin>)>,
    >,
    moves: Query<&Moves>,
    move_list: Option<Res<MoveList>>,
    move_data: Res<Assets<MoveData>>,
    mut query2: Query<&mut NPCDesicion>,
) {
    let Some(move_list) = move_list else {
        return;
    };
    for (entity, npc, player, gtransform) in &query {
        let Some((npc, npc_actions, anim)) = npc else {
            continue;
        };

        // blocking animations shouldnt let them do anything anyways, so skip now to save on the extra work
        if match anim.animation_data() {
            Some(some) => some.is_blocking(),
            None => false,
        } {
            continue;
        }
        let result = match *npc_actions {
            NPCActions::Offensive { focus }
                if !can_aggro(npc, {
                    let get = query.get(focus).unwrap();
                    get.1
                        .map(|npc| npc.0)
                        .or(get.2.map(|_| &NPCInfo::Team {}))
                        .unwrap()
                }) =>
            {
                NPCDesicion::SetAnimation(AnimType::Idle)
            }
            NPCActions::Offensive { focus } => {
                let (_, _, _, other_gtransform) = query.get(focus).unwrap();

                let distance = Vec3::from((
                    other_gtransform.translation().xz() - gtransform.translation().xz(),
                    0.0,
                ));

                if let Some((move_id, range)) =
                    moves.get(entity).unwrap().iter().find_map(|move_id| {
                        let move_data =
                            move_data.get(move_list.data.get(move_id).unwrap()).unwrap();

                        move_data
                            .extra_info
                            .get("range")
                            .and_then(|range| range.clone().into_rust::<f32>().ok())
                            .map(|range| (*move_id, range))
                    })
                {
                    if distance.length_squared() <= range.powi(2) {
                        NPCDesicion::BasicAttack {
                            direction: Dir2::new(distance.normalize_or_zero().xy()).ok(),
                            move_id,
                        }
                    } else {
                        NPCDesicion::Move {
                            target: distance.normalize_or_zero().xzy(),
                        }
                    }
                } else {
                    NPCDesicion::Move {
                        target: distance.normalize_or_zero().xzy(),
                    }
                }
            }
            NPCActions::Defensive => {
                let mut distance = Vec3::ZERO;

                for (_, other_npc, player, other_gtransform) in &query {
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

                NPCDesicion::Move { target: distance }
            }
            NPCActions::Idle { explore } => {
                if explore != Vec3::ZERO {
                    NPCDesicion::Move { target: explore }
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
        &mut AnimationHandler,
        &mut FacingDirection,
        &mut Transform,
    )>,
    time: Res<Time>,
) {
    for (entity, info, desicion, mut anim, mut facing, mut transform) in &mut query {
        match desicion.clone() {
            NPCDesicion::Idle => (),
            NPCDesicion::Move { target: direction } => {
                transform.translation += direction * time.delta_secs();
            }
            NPCDesicion::BasicAttack { direction, move_id } => {
                if let Some(data) = anim.animation_data()
                    && !data.is_blocking()
                {
                    // if let Some(direction) = direction {
                    //     anim.update_direction(direction);
                    // }
                    // in case move does not override animation, provide blocking animation here
                    anim.start_animation(AnimType::AttackTackle);
                    if let Some(dir) = direction {
                        facing.set(dir);
                    }
                    commands.queue(SpawnMove {
                        move_id,
                        parent: entity,
                    })
                }
            }
            NPCDesicion::SetAnimation(anim_type) => {
                anim.start_animation(anim_type);
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
