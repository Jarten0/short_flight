use super::{Client, ClientQuery};
use crate::moves::interfaces::SpawnMove;
use crate::moves::Move;
use crate::npc::animation::NPCAnimation;
use crate::tile::{TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::prelude::*;
use short_flight::animation::{self, cardinal, AnimType};
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
    shaymin_entity: Client,
    shaymin: ClientQuery<
        (&Transform, &mut ShayminRigidbody, Option<&mut NPCAnimation>),
        Without<Camera3d>,
    >,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let (transform, mut rigidbody, anim) = shaymin.into_inner();

    let mut cam_transform = camera.unwrap().into_inner();

    let Some(mut anim) = anim else {
        return;
    };

    if !anim.animation_data().is_blocking() {
        let movement = time.delta_secs() * 30.;
        if kb.just_pressed(KeyCode::KeyK) {
            anim.start_animation(animation::AnimType::AttackShoot, None);
            commands.queue(SpawnMove {
                move_id: Move::MagicalLeaf,
                parent: *shaymin_entity,
            });
            rigidbody.velocity = rigidbody
                .velocity
                .xz()
                .move_towards(Vec2::ZERO, movement * 2.)
                .xxy()
                .with_y(rigidbody.velocity.y);
        } else {
            let input = get_input(kb).normalize_or_zero();
            rigidbody.velocity = rigidbody
                .velocity
                .xz()
                .move_towards(input.xz() * 1.5, movement)
                .xxy()
                .with_y(rigidbody.velocity.y);
            if input.length_squared() > 0.0 {
                let input = Dir2::new(input.xz().normalize_or(Vec2::NEG_Y)).unwrap();

                let new_cardinal = cardinal(input) != cardinal(anim.direction())
                    || anim.current() != AnimType::Walking;

                if new_cardinal {
                    anim.start_animation(animation::AnimType::Walking, Some(input));
                    anim.loop_ = true;
                } else if rigidbody.velocity == Vec3::ZERO {
                    anim.start_animation(animation::AnimType::Idle, Some(input));
                } else {
                    anim.loop_ = true;
                }
            } else {
                anim.start_animation(AnimType::Idle, None);
                anim.loop_ = false;
            }
        }
    }

    cam_transform.translation = transform.translation.with_y(transform.translation.y + 10.);
}

pub fn get_input(kb: Res<ButtonInput<KeyCode>>) -> Vec3 {
    let mut dir: Vec3 = Vec3::ZERO;

    if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::Space) {
        return dir;
    }

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
    gizmos.cross(transform.translation, 2., palettes::basic::NAVY);
}
