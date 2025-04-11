use short_flight::animation::AnimType;
use short_flight::collision::{BasicCollider, CollisionLayers, DynamicCollision, ZHitbox};

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
                CollisionLayers::NPC,
                CollisionLayers::NPC,
            ),
            ZHitbox {
                y_tolerance: 1.0,
                neg_y_tolerance: 0.0,
            },
            DynamicCollision::default(),
            Self,
        ));
        let mut anim = world
            .get_mut::<AnimationHandler>(Self::parent(&world, entity))
            .unwrap();
        anim.start_animation(AnimType::AttackTackle, None);
    }
}

fn tackle(
    active_moves: Query<(&Tackle, &Parent)>,
    mut parent: Query<(&mut Transform, &AnimationHandler)>,
    time: Res<Time>,
) {
    for (tackle, entity) in active_moves.iter() {
        let (mut transform, anim) = parent.get_mut(**entity).unwrap();

        match anim.frame() / anim.speed() {
            0.0..2.0 => {
                transform.translation += (anim.direction() * time.delta_secs() * 4.0)
                    .xxy()
                    .with_y(0.0);
            }
            2.0..3.0 => {
                transform.translation += (anim.direction() * time.delta_secs() * 0.5)
                    .xxy()
                    .with_y(0.0);
            }
            _ => (),
        }
    }
}
