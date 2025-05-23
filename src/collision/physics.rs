use super::{CollisionExitEvent, TilemapCollision};
use crate::collision::{BasicCollider, CollisionEnterEvent, CollisionLayers, DynamicCollision};
use crate::ldtk::TileQuery;
use crate::npc::stats::{Damage, Health};
use crate::tile::{TileDepth, TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

#[derive(Debug, Component)]
#[require(DynamicCollision)]
pub struct Rigidbody {
    pub ground: HashSet<Entity>,
    pub wall: HashSet<Entity>,
    pub velocity: Vec3,
    pub previous_position: Vec3,
    pub last_push: Vec3,
}

pub fn update_dynamic_collision(mut dyn_collision: Query<(&mut Rigidbody, &GlobalTransform)>) {
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
            &mut Rigidbody,
            &mut Transform,
            &GlobalTransform,
            &BasicCollider,
        ),
        With<DynamicCollision>,
    >,
    tile_data_query: Query<(&GlobalTransform, &TileSlope, &TileFlags), Without<Rigidbody>>,
    time: Res<Time>,
) {
    for (mut rigidbody, mut transform, gtransform, basic_collider) in &mut dyn_collision {
        transform.translation += rigidbody.velocity * time.delta_secs();

        let grounded = rigidbody
            .ground
            .iter()
            .filter_map(|entity| tile_data_query.get(*entity).ok())
            .reduce(|(g, ts, tf), (g2, ts2, tf2)| {
                if ts.get_slope_height(tf) + g.translation().y
                    >= ts2.get_slope_height(tf2) + g2.translation().y
                {
                    (g, ts, tf)
                } else {
                    (g2, ts2, tf2)
                }
            });

        if let Some((gtransform2, slope, flags)) = grounded {
            let slope_height = slope.get_height_at_point(
                flags,
                gtransform.translation().xz() - gtransform2.translation().xz(),
            );
            transform.translation.y = transform.translation.y.clamp(
                gtransform2.translation().y + slope_height,
                gtransform2.translation().y + slope_height,
            );

            rigidbody.velocity.y = 0.0;
        } else {
            const GRAVITY: f32 = 2.0;
            rigidbody.velocity.y -= GRAVITY * time.delta_secs();
        }
    }
}

/// If observing, then the entity will be pushed outside of tilemaps
pub fn move_out_from_tilemaps(
    trigger: Trigger<CollisionEnterEvent>,
    mut rigidbody: Query<(&mut Rigidbody, &GlobalTransform, &mut Transform)>,
    other_col: Query<(&GlobalTransform, &TileSlope, &TileFlags), With<TileDepth>>,
    tile_query: TileQuery,
) {
    let Ok((mut rigidbody, this_transform, mut transform)) = rigidbody.get_mut(trigger.this) else {
        log::info!("No rigidbody found for {}", trigger.this);
        return;
    };

    let Ok((tile_transform, tile_slope, tile_flags)) = other_col.get(trigger.other) else {
        log::info!("No tile collider info found for {}", trigger.other);
        return;
    };

    let position = this_transform.translation();
    let other_pos = tile_transform.translation();
    let relative_pos = position - other_pos;

    let point =
        this_transform.translation().xz() - (tile_transform.translation().xz() - (Vec2::ONE / 2.));
    let movement = position - rigidbody.previous_position;

    if rigidbody.previous_position.y >= other_pos.y - 0.1 {
        rigidbody.ground.insert(trigger.other);
        let slope_height = tile_slope.get_height_at_point(tile_flags, point);

        transform.translation.y = transform
            .translation
            .y
            .clamp(other_pos.y + slope_height, f32::INFINITY);
    } else {
        rigidbody.wall.insert(trigger.other);
        let inverse_movement_dir: Dir2 =
            Dir2::new(relative_pos.xz().normalize_or_zero()).unwrap_or(Dir2::X);
        // -Dir2::new(movement.xz().normalize_or_zero()).unwrap_or(Dir2::X);

        let push = if inverse_movement_dir.x.abs() == inverse_movement_dir.y.abs() {
            Vec2::X
        } else if inverse_movement_dir.x.abs() > inverse_movement_dir.y.abs() {
            Vec2::X * inverse_movement_dir.x.signum()
        } else {
            Vec2::Y * inverse_movement_dir.y.signum()
        };

        rigidbody.last_push = -push.extend(0.0).xzy();

        transform.translation += push.extend(0.0).xzy();

        // rigidbody.velocity = rigidbody
        //     .velocity
        //     .xz()
        //     .clamp(push, push)
        //     .extend(rigidbody.velocity.y)
        //     .xzy();
    }
}

pub fn unground_on_leave(
    trigger: Trigger<CollisionExitEvent>,
    mut rigidbody: Query<(&mut Rigidbody, &GlobalTransform, &mut Transform)>,
) {
    rigidbody
        .get_mut(trigger.this)
        .expect("Rigidbody was despawned??")
        .0
        .ground
        .remove(&trigger.other);
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
