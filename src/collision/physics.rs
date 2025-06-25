use super::{CollisionExitEvent, TilemapCollision};
use crate::collision::{BasicCollider, CollisionEnterEvent, CollisionLayers, DynamicCollision};
use crate::ldtk::TileQuery;
use crate::npc::stats::{Damage, Health};
use crate::tile::{TileDepth, TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

#[derive(Debug, Component, Default)]
#[require(DynamicCollision)]
pub struct Rigidbody {
    pub ground: HashSet<Entity>,
    pub wall: HashSet<Entity>,
    pub velocity: Vec3,
    pub previous_position: Vec3,
    pub last_push: Vec3,
}

pub struct TilemapRigidbody {
    pub ground: HashSet<Entity>,
    pub wall: HashSet<Entity>,
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
    mut gizmos: Gizmos,
    kb: Res<ButtonInput<KeyCode>>,
) {
    let draw_gizmos = kb.pressed(KeyCode::KeyV);
    for (mut rigidbody, mut transform, gtransform, basic_collider) in &mut dyn_collision {
        transform.translation += rigidbody.velocity * time.delta_secs();

        let grounded = rigidbody
            .ground
            .iter()
            .filter_map(|entity| tile_data_query.get(*entity).ok())
            .reduce(|(transform, slope, flags), (transform2, slope2, flags2)| {
                let point = gtransform.translation().xz();
                if transform.translation().xz().distance_squared(point)
                    <= transform2.translation().xz().distance_squared(point)
                {
                    (transform, slope, flags)
                } else {
                    (transform2, slope2, flags2)
                }
            });

        if let Some((gtransform2, slope, flags)) = grounded {
            let slope_height = gtransform2.translation().y
                + slope.get_height_at_point(
                    flags,
                    gtransform.translation().xz() - gtransform2.translation().xz(),
                );

            if draw_gizmos {
                gizmos.rect(
                    Isometry3d::new(
                        gtransform2.translation().with_y(slope_height + 0.1)
                            + Vec3 {
                                x: 0.5,
                                y: 0.0,
                                z: 0.5,
                            },
                        Quat::from_rotation_x(f32::to_radians(-90.0)),
                    ),
                    Vec2::ONE,
                    palettes::basic::PURPLE,
                );
            }

            if transform.translation.y + rigidbody.velocity.y <= slope_height {
                transform.translation.y = slope_height;
                rigidbody.velocity.y = rigidbody.velocity.y.clamp(0., f32::INFINITY);
            }

            // transform.translation.y = transform.translation.y.clamp(
            //     gtransform2.translation().y + slope_height,
            //     gtransform2.translation().y + slope_height,
            // );
        } else {
            const GRAVITY: f32 = 2.0;
            rigidbody.velocity.y -= GRAVITY * time.delta_secs();
        }
        for (transform2, slope, flags) in rigidbody
            .wall
            .iter()
            .filter_map(|entity| tile_data_query.get(*entity).ok())
        {
            let translation = transform2.translation();
        }
    }
}

/// If observing, then the entity will be pushed outside of tilemaps
pub fn move_out_from_tilemaps(
    trigger: Trigger<CollisionEnterEvent>,
    mut rigidbody: Query<(&mut Rigidbody, &GlobalTransform, &mut Transform)>,
    other_col: Query<(&GlobalTransform, &TileSlope, &TileFlags), With<TileDepth>>,
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

    let slope_height = tile_slope.get_height_at_point(tile_flags, point);

    if rigidbody.previous_position.y >= slope_height - 0.2 {
        rigidbody.ground.insert(trigger.other);

        transform.translation.y = transform
            .translation
            .y
            .clamp(other_pos.y + slope_height, f32::INFINITY);
    } else {
        rigidbody.wall.insert(trigger.other);

        let wall_pos = other_pos.xz() + (Vec2::ONE / 2.);
        let relative_pos = position.xz() - wall_pos;

        let wall_normal: Vec2 = if relative_pos.x.abs() > relative_pos.y.abs() {
            Vec2::X * relative_pos.x.signum()
        } else if relative_pos.y.abs() > relative_pos.x.abs() {
            Vec2::Y * relative_pos.y.signum()
        } else {
            Vec2::X
        };

        // let push = relative_pos - wall_normal;
        let push = -((relative_pos.length() * wall_normal.normalize_or_zero()) - wall_normal);

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
    let mut rigidbody = rigidbody
        .get_mut(trigger.this)
        .expect("Rigidbody was despawned??")
        .0;
    rigidbody.ground.remove(&trigger.other);
    rigidbody.wall.remove(&trigger.other);
}

/// If observed by an entity, this entity will collide with projectiles and attacks
pub fn take_hits(
    trigger: Trigger<CollisionEnterEvent>,
    mut this_rigidbody: Query<&mut Health>,
    other_col: Query<&BasicCollider>,
    other_query: Query<&Damage>,
) {
    let Ok((mut health)) = this_rigidbody.get_mut(trigger.this) else {
        return;
    };

    let Ok((colliding)) = other_col.get(trigger.other) else {
        return;
    };

    if !colliding
        .layers
        .intersects(CollisionLayers::Projectile | CollisionLayers::Attack)
    {
        return;
    }

    let Ok((damage)) = other_query.get(trigger.other) else {
        return;
    };

    health.hp -= **damage;
}
