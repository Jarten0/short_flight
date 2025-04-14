use crate::collision::{BasicCollider, CollisionEnterEvent, CollisionLayers, DynamicCollision};
use crate::npc::stats::{Damage, Health};
use crate::tile::{TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::prelude::*;
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
