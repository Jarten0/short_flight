use short_flight::animation::AnimType;

use crate::npc::animation::NPCAnimation;

use super::prelude::*;
#[derive(Component, Reflect)]
pub(super) struct MagicalLeaf;

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
    }
}

fn process(query: Query<&MagicalLeaf>, parent: Query<&mut NPCAnimation>) {}
