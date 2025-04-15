use crate::collision::{BasicCollider, CollisionEnterEvent, CollisionLayers, DynamicCollision};
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

pub fn update_dynamic_collision(mut dyn_collision: Query<(&mut DynamicCollision, &Transform)>) {
    for (mut dyn_info, transform) in &mut dyn_collision {
        if dyn_info.previous_position != transform.translation {
            dyn_info.previous_position = transform.translation;
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
    mut gizmos: Gizmos,
) {
    let Ok((_rigidbody, gtransform, mut transform)) = rigidbody.get_mut(trigger.this) else {
        return;
    };

    let Ok((colliding, gtransform2)) = other_col.get(trigger.other) else {
        return;
    };

    if !colliding.layers.intersects(CollisionLayers::Wall) {
        return;
    }

    let Ok((tile_slope, tile_flags)) = tile_data_query.get(trigger.other) else {
        return;
    };

    let point = gtransform.translation().xz() - gtransform2.translation().xz();

    let max = tile_slope.get_height_at_point(tile_flags, point);

    transform.translation.y = max + gtransform2.translation().y;
    gizmos.cross(transform.translation, 2., palettes::basic::NAVY);
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
