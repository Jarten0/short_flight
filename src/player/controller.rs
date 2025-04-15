use super::{Client, ClientQuery};
use crate::animation::{self, cardinal, AnimType};
use crate::collision::physics::RigidbodyProperties;
use crate::collision::{
    self, BasicCollider, ColliderShape, CollisionEnterEvent, CollisionExitEvent, CollisionLayers,
    DynamicCollision, StaticCollision, ZHitbox,
};
use crate::moves::interfaces::SpawnMove;
use crate::moves::Move;
use crate::npc::animation::AnimationHandler;
use crate::npc::stats::FacingDirection;
use crate::tile::{TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::prelude::*;

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
        RigidbodyProperties {
            grounded: None,
            velocity: Vec3::default(),
        },
        DynamicCollision::default(),
    ));
    commands
        .entity(*shaymin)
        .observe(collision::physics::move_out_from_tilemaps)
        .observe(
            |trigger: Trigger<CollisionEnterEvent>, mut query: Query<&mut RigidbodyProperties>| {
                let Ok(mut rigidbody) = query.get_mut(trigger.this) else {
                    return;
                };
                rigidbody.grounded = Some(trigger.other);
            },
        )
        .observe(
            |trigger: Trigger<CollisionExitEvent>, mut query: Query<&mut RigidbodyProperties>| {
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
        (
            &Transform,
            &mut RigidbodyProperties,
            Option<&mut AnimationHandler>,
            &mut FacingDirection,
        ),
        Without<Camera3d>,
    >,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut commands: Commands,
    mut gizmos: Gizmos,
) {
    let (transform, mut rigidbody, anim, mut facing) = shaymin.into_inner();

    let mut cam_transform = camera.unwrap().into_inner();

    let Some(mut anim) = anim else {
        return;
    };

    if !anim.animation_data().is_blocking() {
        let movement = time.delta_secs() * 30.;
        if kb.just_pressed(KeyCode::KeyK) {
            anim.start_animation(animation::AnimType::AttackShoot);
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

                let mut rhs = *input;
                if **facing == -input {
                    rhs -= Vec2::NEG_ONE;
                }

                let target = facing
                    .move_towards(
                        rhs,
                        time.delta_secs() * facing.distance_squared(*input) * 8.,
                    )
                    .normalize_or_zero();

                gizmos.arrow(
                    transform.translation,
                    transform.translation + Vec3::from((*input, 1.0)).xzy(),
                    palettes::basic::GREEN,
                );
                gizmos.arrow(
                    transform.translation,
                    transform.translation + Vec3::from((target, 1.0)).xzy(),
                    palettes::basic::YELLOW,
                );

                let new_dir = Dir2::new(target).unwrap_or(*FacingDirection::default());

                let new_cardinal =
                    cardinal(input) != cardinal(**facing) || anim.current() != AnimType::Walking;

                facing.set(new_dir);
                if new_cardinal {
                    anim.start_animation(animation::AnimType::Walking);
                    anim.looping = true;
                } else if rigidbody.velocity == Vec3::ZERO {
                    anim.start_animation(animation::AnimType::Idle);
                } else {
                    anim.looping = true;
                }
            } else {
                anim.start_animation(AnimType::Idle);
                anim.looping = false;
            }
        }
    }

    gizmos.arrow(
        transform.translation,
        transform.translation + Vec3::from((***facing, 1.0)).xzy(),
        palettes::basic::RED,
    );

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
