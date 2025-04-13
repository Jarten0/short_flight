use crate::animation::AnimType;

use crate::npc::animation::AnimationHandler;
use crate::npc::stats::{Damage, FacingDirection};
use crate::projectile::interfaces::SpawnProjectile;
use crate::projectile::Projectile;

use super::prelude::*;
#[derive(Component, Reflect)]
pub struct MagicalLeaf;

impl MoveComponent for MagicalLeaf {
    fn build(&mut self, app: &mut App) {
        // app.add_systems(FixedUpdate, process);
    }

    // fn variant(&self) -> super::Move
    // where
    //     Self: Sized,
    // {
    //     super::Move::MagicalLeaf
    // }

    fn on_spawn(
        &mut self,
        world: &mut World,
        move_entity: Entity,
        move_data: &super::interfaces::MoveData,
    ) {
        let parent = world.get::<Parent>(move_entity).unwrap().get();
        let position = world.get::<GlobalTransform>(parent).unwrap().translation();
        let direction = **world.get::<FacingDirection>(parent).unwrap();
        world.entity_mut(move_entity).insert((Self, Damage(20)));
        Self::set_animation(world, move_entity, AnimType::AttackShoot);
        world.commands().queue(SpawnProjectile {
            source: Some(move_entity),
            projectile_id: Projectile::LeafAttack,
            position,
            direction,
        });
    }
}

// fn process(
//     mut query: Query<(&MagicalLeaf, &mut Transform, &Parent)>,
//     parent: Query<&mut AnimationHandler>,
// ) {
//     for (_, mut transform, parent_id) in &mut query {
//         let Ok(anim) = parent.get(parent_id.get()) else {
//             continue;
//         };
//     }
// }
