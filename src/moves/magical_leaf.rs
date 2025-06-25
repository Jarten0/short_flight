use super::interfaces::MoveData;
use super::prelude::*;
use crate::animation::{AnimType, AnimationData, AnimationDirLabel};
use crate::assets::AnimationSpritesheet;
use crate::npc::animation::AnimationHandler;
use crate::npc::stats::{Damage, FacingDirection};
use crate::projectile::Projectile;
use crate::projectile::interfaces::{ProjectileCatalog, SpawnProjectile};
use bevy::platform::collections::HashMap;

#[derive(Component, Reflect)]
pub struct MagicalLeaf;

impl MoveComponent for MagicalLeaf {
    fn build(&mut self, app: &mut App) {
        // let world = app.world_mut();
        // let move_data = MoveData {
        //     display_name: String::from("Magical Leaf"),
        //     spritesheet: Some(AnimationSpritesheet::new(
        //         Vec::from([(
        //             AnimType::Idle,
        //             AnimationData {
        //                 variant: AnimType::Idle,
        //                 frames: 1,
        //                 direction_label: AnimationDirLabel::None,
        //                 blocking_override: None,
        //             },
        //         )]),
        //         UVec2 { x: 32, y: 32 },
        //         world
        //             .resource::<ProjectileCatalog>()
        //             .image_files
        //             .get(&Projectile::LeafAttack)
        //             .unwrap()
        //             .clone(),
        //         world.resource::<AssetServer>(),
        //     )),
        //     collider: None,
        //     related_animaition: None,
        //     extra_info: HashMap::new(),
        // };
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
        let parent = world.get::<ChildOf>(move_entity).unwrap().parent();
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
        world.flush()
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
