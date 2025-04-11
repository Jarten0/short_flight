use short_flight::animation::AnimType;

use crate::npc::animation::AnimationHandler;
use crate::projectile::interfaces::SpawnProjectile;
use crate::projectile::Projectile;

use super::prelude::*;
#[derive(Component, Reflect)]
pub struct MagicalLeaf;

impl MoveComponent for MagicalLeaf {
    fn build(&mut self, app: &mut App) {
        app.add_systems(FixedUpdate, process);
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
        entity: Entity,
        move_data: &super::interfaces::MoveData,
    ) {
        world.entity_mut(entity).insert(Self);
        Self::set_animation(world, entity, AnimType::AttackShoot, None);
        world.commands().queue(SpawnProjectile {
            source: entity,
            projectile_id: Projectile::LeafAttack,
        });
    }
}

fn process(
    mut query: Query<(&MagicalLeaf, &mut Transform, &Parent)>,
    parent: Query<&mut AnimationHandler>,
) {
    for (_, mut transform, parent_id) in &mut query {
        let Ok(anim) = parent.get(parent_id.get()) else {
            continue;
        };
    }
}
