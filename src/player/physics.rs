use std::cmp::Ordering;

use super::anim_state::ShayminAnimation;
use super::{Client, ClientQuery};
use crate::tile::{TileDepth, TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;
use short_flight::animation;
use short_flight::collision::{
    BasicCollider, ColliderShape, CollisionEnterEvent, CollisionExitEvent, CollisionLayers,
    DynamicCollision, StaticCollision, ZHitbox,
};

#[derive(Debug, Component)]
#[require(DynamicCollision)]
pub struct ShayminRigidbody {
    grounded: Option<Entity>,
    velocity: Vec3,
}

pub fn setup(shaymin: Client, mut commands: Commands) {
    commands.entity(*shaymin).insert((
        BasicCollider::new(
            true,
            ColliderShape::Circle(20. / 32. / 2.),
            CollisionLayers::NPC,
            CollisionLayers::NPC | CollisionLayers::Projectile | CollisionLayers::Wall,
        ),
        ZHitbox {
            y_tolerance: 0.5,
            neg_y_tolerance: 0.0,
        },
        ShayminRigidbody {
            grounded: None,
            velocity: Vec3::default(),
        },
        DynamicCollision::default(),
    ));
    commands
        .entity(*shaymin)
        .observe(move_out_from_tilemaps)
        .observe(
            |trigger: Trigger<CollisionEnterEvent>, mut query: Query<&mut ShayminRigidbody>| {
                let Ok(mut rigidbody) = query.get_mut(trigger.this) else {
                    return;
                };
                rigidbody.grounded = Some(trigger.other);
            },
        )
        .observe(
            |trigger: Trigger<CollisionExitEvent>, mut query: Query<&mut ShayminRigidbody>| {
                let Ok(mut rigidbody) = query.get_mut(trigger.this) else {
                    return;
                };
                rigidbody.grounded = None;
            },
        );
}

pub fn control_shaymin(
    shaymin: ClientQuery<(&mut Transform, Option<&mut ShayminAnimation>), Without<Camera3d>>,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    delta: Res<Time<Fixed>>,
) {
    let (mut transform, anim) = shaymin.into_inner();

    let mut cam_transform = camera.unwrap().into_inner();

    let Some(mut anim) = anim else {
        return;
    };

    let current = anim.current;
    let animation = anim.pool.get_mut(&current).unwrap();

    if !animation.is_blocking() {
        if let Some(movement) = manage_movement(kb, &mut transform, &delta) {
            anim.direction = movement.xy();
            if current == animation::Idle {
                anim.current = animation::Walking;
            }
        };
    }

    cam_transform.translation = {
        let mut vec3 = transform.translation;
        vec3.y += 10.0;
        vec3
    };
}

pub fn manage_movement(
    kb: Res<ButtonInput<KeyCode>>,
    transform: &mut Mut<Transform>,
    delta: &Res<Time<Fixed>>,
) -> Option<Vec3> {
    if kb.pressed(KeyCode::ShiftLeft) {
        return None;
    }
    if kb.pressed(KeyCode::Space) {
        return None;
    }

    let input = {
        let mut dir: Vec3 = Vec3::ZERO;
        if kb.pressed(KeyCode::KeyA) {
            dir += Vec3::NEG_X
        }
        if kb.pressed(KeyCode::KeyD) {
            dir += Vec3::X
        }
        if kb.pressed(KeyCode::KeyW) {
            dir += Vec3::NEG_Z
        }
        if kb.pressed(KeyCode::KeyS) {
            dir += Vec3::Z
        }

        dir
    };
    let movement = input * 1.5 * delta.delta_secs();
    transform.translation += movement;
    return Some(movement);
}

pub fn draw_colliders(
    rigidbodies: Query<(
        &BasicCollider,
        AnyOf<(&DynamicCollision, &StaticCollision)>,
        &GlobalTransform,
        &ZHitbox,
    )>,
    mut gizmos: Gizmos,
) {
    for (collider, (dyn_info, stat_info), transform, zhitbox) in &rigidbodies {
        let color = match (dyn_info, stat_info) {
            (Some(_), _) => palettes::basic::AQUA,
            (_, Some(_)) => palettes::basic::FUCHSIA,
            _ => palettes::basic::RED,
        };
        let mut translation = transform.translation();
        translation.y += zhitbox.neg_y_tolerance;
        let mut translation2 = transform.translation();
        translation2.y += zhitbox.y_tolerance;

        if let ColliderShape::Circle(radius) = collider.shape {
            gizmos.circle(
                Isometry3d::new(translation, Quat::from_rotation_x(f32::to_radians(90.0))),
                radius,
                color,
            );
            gizmos.circle(
                Isometry3d::new(translation2, Quat::from_rotation_x(f32::to_radians(90.0))),
                radius,
                color.with_green(0.5),
            );
        }
        if let ColliderShape::Rect(rect) = collider.shape {
            gizmos.rect(
                Isometry3d::new(
                    translation + Vec3::new(rect.center().x, -0.01, rect.center().y),
                    Quat::from_rotation_x(f32::to_radians(90.0)),
                ),
                rect.size(),
                color,
            );
            gizmos.rect(
                Isometry3d::new(
                    translation2 + Vec3::new(rect.center().x, 0.02, rect.center().y),
                    Quat::from_rotation_x(f32::to_radians(90.0)),
                ),
                rect.size(),
                color.with_green(0.5),
            );
        }
    }
}

pub fn update_dynamic_collision(mut dyn_collision: Query<(&mut DynamicCollision, &Transform)>) {
    for (mut dyn_info, transform) in &mut dyn_collision {
        if dyn_info.previous_position != transform.translation {
            dyn_info.previous_position = transform.translation;
        }
    }
}

pub fn update_rigidbodies(
    mut dyn_collision: Query<(
        &mut ShayminRigidbody,
        &mut Transform,
        &GlobalTransform,
        &BasicCollider,
    )>,
    tile_data_query: Query<(&GlobalTransform, &TileSlope, &TileFlags), Without<ShayminRigidbody>>,
    time: Res<Time>,
) {
    for (mut rigidbody, mut transform, gtransform, basic_collider) in &mut dyn_collision {
        if let Some(ground) = rigidbody.grounded {
            let first = basic_collider
                .currently_colliding
                .iter()
                .filter_map(|value| tile_data_query.get(*value).ok())
                .max_by(|item, item2| item.0.translation().y.total_cmp(&item2.0.translation().y));
            let Some((gtransform2, slope, flags)) = first else {
                rigidbody.grounded = None;
                continue;
            };

            transform.translation.y = gtransform2.translation().y
                + slope.get_height_at_point(
                    flags,
                    gtransform.translation().xz() - gtransform2.translation().xz(),
                );
        }

        match rigidbody.grounded {
            Some(s) => {
                rigidbody.velocity.y = 0.0;
            }
            None => {
                rigidbody.velocity.y -= 2.0 * time.delta_secs();
            }
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
) {
    let Ok((rigidbody, gtransform, mut transform)) = rigidbody.get_mut(trigger.this) else {
        return;
    };

    let Ok((collider, gtransform2)) = other_col.get(trigger.other) else {
        return;
    };

    if !collider.layers.intersects(CollisionLayers::Wall) {
        return;
    }

    let Ok((tile_slope, tile_flags)) = tile_data_query.get(trigger.other) else {
        return;
    };

    let point = gtransform.translation().xz() - gtransform2.translation().xz();

    let max = tile_slope.get_height_at_point(tile_flags, point);

    transform.translation.y = max + gtransform2.translation().y;
}
