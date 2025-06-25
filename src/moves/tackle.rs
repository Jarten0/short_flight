use crate::animation::AnimType;
use crate::collision::physics::Rigidbody;
use crate::collision::{BasicCollider, CollisionLayers, DynamicCollision, ZHitbox};
use crate::npc::stats::FacingDirection;

use super::interfaces::MoveData;
use super::prelude::*;
use crate::npc::animation::AnimationHandler;

#[derive(Component, Reflect)]
pub(crate) struct Tackle;

impl MoveComponent for Tackle {
    fn build(&mut self, app: &mut App) {
        app.add_systems(FixedUpdate, tackle);
    }

    fn on_spawn(&mut self, world: &mut World, entity: Entity, move_data: &MoveData) {
        world.entity_mut(entity).insert((
            BasicCollider::new(
                true,
                move_data.collider.clone().unwrap(),
                CollisionLayers::Attack,
                CollisionLayers::NPC,
            ),
            ZHitbox {
                y_tolerance: 1.0,
                neg_y_tolerance: 0.0,
            },
            DynamicCollision::default(),
            Rigidbody::default(),
            Self,
        ));
        let mut anim = world
            .get_mut::<AnimationHandler>(Self::parent(&world, entity))
            .unwrap();
        anim.start_animation(AnimType::AttackTackle);
    }
}

fn tackle(
    active_moves: Query<(&Tackle, &ChildOf)>,
    mut parent_query: Query<(
        &mut Transform,
        &AnimationHandler,
        &FacingDirection,
        &mut Rigidbody,
    )>,
    time: Res<Time>,
) {
    for (_move, child) in active_moves.iter() {
        let (mut transform, anim, dir, mut rigidbody) =
            parent_query.get_mut(child.parent()).unwrap();

        let frame = anim.frame() / anim.speed();
        match frame {
            0.0..2.0 => {
                rigidbody.velocity += (**dir * time.delta_secs() * (4.0 * 4.0 / (frame + 1.0)))
                    .extend(0.)
                    .xzy();

                rigidbody.velocity.y = 1.5 - frame;
            }
            2.0..3.0 => {
                rigidbody.velocity = (rigidbody.velocity.xz().normalize() * 0.2)
                    .extend(rigidbody.velocity.y)
                    .xzy();
                // transform.translation += (**dir * time.delta_secs() * 0.5).xxy().with_y(0.0);
            }
            _ => (),
        }
    }
}
