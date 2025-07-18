use crate::assets::AnimationSpritesheet;
use crate::moves::magical_leaf::MagicalLeaf;
use crate::npc::animation::AnimationHandler;
use crate::npc::stats::{Damage, FacingDirection};

use super::interfaces::{ProjectileCatalog, ProjectileData};
use super::{Projectile, ProjectileInterface};
use crate::collision::{BasicCollider, ColliderShape};
use bevy::prelude::*;

#[derive(Component)]
pub struct LeafAttack;

impl ProjectileInterface for LeafAttack {
    fn build(&mut self, app: &mut App) {
        app.add_systems(FixedUpdate, process);
        // let world = app.world_mut();
        // let projectile_data = ProjectileData {
        //     variant: Projectile::LeafAttack,
        //     display_name: String::from("Leaves"),
        //     spritesheet: AnimationSpritesheet::new(
        //         Vec::new(),
        //         UVec2 { x: 32, y: 32 },
        //         world
        //             .resource::<ProjectileCatalog>()
        //             .image_files
        //             .get(&Projectile::LeafAttack)
        //             .unwrap()
        //             .clone(),
        //         world.resource::<AssetServer>(),
        //     ),
        //     collider: ColliderShape::default(),
        //     damage: Damage::default(),
        //     assets: None,
        // };
        // let handle = world.resource::<AssetServer>().add(projectile_data);
        // world
        //     .resource_mut::<ProjectileCatalog>()
        //     .data_files
        //     .insert(Projectile::LeafAttack, handle);
    }

    fn on_spawn(
        &mut self,
        world: &mut World,
        projectile_entity: Entity,
        source: Option<Entity>,
        projectile_data: &super::interfaces::ProjectileData,
    ) {
        if let Some(source) = source {
            let (magical_leaf, damage) = world
                .query::<(&MagicalLeaf, &Damage)>()
                .get(world, source)
                .unwrap();
            let damage = damage.clone();
            world.entity_mut(projectile_entity).insert((Self, damage));
        }
    }
}

pub fn process(
    mut query: Query<(
        Entity,
        &LeafAttack,
        &mut BasicCollider,
        &mut Transform,
        &FacingDirection,
    )>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, _leaf_attack, mut collider, mut transform, anim) in &mut query {
        if let ColliderShape::Circle(radius) = &mut collider.shape {
            *radius -= time.delta_secs();
            if *radius <= 0.0 {
                commands.entity(entity).despawn();
            }
        }

        transform.translation += anim.xxy().with_y(0.0) * time.delta_secs() * 8.0;
    }
}
