use crate::collision::{BasicCollider, CollisionEnterEvent, CollisionLayers, DynamicCollision};
use crate::ldtk::TileQuery;
use crate::npc::stats::{Damage, Health};
use crate::tile::{TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::prelude::*;

#[derive(Debug, Component)]
#[require(DynamicCollision)]
pub struct RigidbodyProperties {
    pub grounded: Option<Entity>,
    pub velocity: Vec3,
}

pub fn update_dynamic_collision(
    mut dyn_collision: Query<(&mut DynamicCollision, &GlobalTransform)>,
) {
    for (mut dyn_info, transform) in &mut dyn_collision {
        let translation = transform.translation();
        if dyn_info.previous_position != translation {
            dyn_info.previous_position = translation;
        }
    }
}

pub fn update_rigidbodies(
    mut dyn_collision: Query<
        (
            &mut RigidbodyProperties,
            &mut Transform,
            &GlobalTransform,
            &BasicCollider,
        ),
        With<DynamicCollision>,
    >,
    tile_data_query: Query<
        (&GlobalTransform, &TileSlope, &TileFlags),
        Without<RigidbodyProperties>,
    >,
    time: Res<Time>,
) {
    for (mut rigidbody, mut transform, gtransform, basic_collider) in &mut dyn_collision {
        let tallest_entity = basic_collider
            .currently_colliding
            .iter()
            .filter_map(|value| tile_data_query.get(*value).ok())
            .max_by(|item, item2| item.0.translation().y.total_cmp(&item2.0.translation().y));

        if let Some((gtransform2, slope, flags)) = tallest_entity {
            transform.translation.y = gtransform2.translation().y
                + slope.get_height_at_point(
                    flags,
                    gtransform.translation().xz() - gtransform2.translation().xz(),
                );

            rigidbody.velocity.y = 0.0;
        } else {
            rigidbody.velocity.y -= 2.0 * time.delta_secs();
        }
        transform.translation += rigidbody.velocity * time.delta_secs();
    }
}

/// If observing, then the entity will be pushed outside of tilemaps
pub fn move_out_from_tilemaps(
    trigger: Trigger<CollisionEnterEvent>,
    mut rigidbody: Query<(&DynamicCollision, &GlobalTransform, &mut Transform)>,
    other_col: Query<(&BasicCollider, &GlobalTransform)>,
    tile_data_query: Query<(&TileSlope, &TileFlags)>,
    tile_query: TileQuery,
) {
    let Ok((rigidbody, this_transform, mut transform)) = rigidbody.get_mut(trigger.this) else {
        return;
    };

    let Ok((colliding, tilemap_transform)) = other_col.get(trigger.other) else {
        return;
    };

    if !colliding.layers.intersects(CollisionLayers::Wall) {
        return;
    }

    let Some(current_tile) = tile_query.get_tile(this_transform.translation()) else {
        return;
    };

    let Ok((tile_slope, tile_flags)) = tile_data_query.get(current_tile) else {
        return;
    };

    let translation = this_transform.translation();
    let other_translation = this_transform.translation();

    if rigidbody.previous_position.y > other_translation.y {
        let point = this_transform.translation().xz()
            - (tilemap_transform.translation().xz() - (Vec2::ONE / 2.));

        let max = tile_slope.get_height_at_point(tile_flags, point);

        transform.translation.y = max + tilemap_transform.translation().y;
    }
}

/// If observed by an entity, this entity will collide with projectiles and attacks
pub fn take_hits(
    trigger: Trigger<CollisionEnterEvent>,
    mut this_rigidbody: Query<(&mut Health)>,
    other_col: Query<(&BasicCollider)>,
    other_query: Query<(&Damage)>,
) {
    let Ok((mut health)) = this_rigidbody.get_mut(trigger.this) else {
        return;
    };

    let Ok((colliding)) = other_col.get(trigger.other) else {
        return;
    };

    if !colliding.layers.intersects(CollisionLayers::Projectile) {
        return;
    }

    let Ok((damage)) = other_query.get(trigger.other) else {
        return;
    };

    health.hp -= **damage;
}
