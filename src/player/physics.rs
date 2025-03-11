use super::anim_state::ShayminAnimation;
use super::{Client, ClientQuery};
use bevy::prelude::*;
use short_flight::animation;
use short_flight::collision::{Collider, ColliderShape, CollisionEvent, DynamicCollision, ZHitbox};

#[derive(Debug, Component)]
#[require(DynamicCollision)]
pub struct ShayminRigidbody {}

pub fn setup(shaymin: Client, mut commands: Commands) {
    commands.entity(*shaymin).insert((
        Collider {
            dynamic: true,
            shape: ColliderShape::Circle { radius: 32. / 20. },
        },
        ZHitbox {
            y_tolerance: 0.5,
            neg_y_tolerance: 0.0,
        },
    ));
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
    let data = anim.pool.get_mut(&current).unwrap();

    if data.can_move() {
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
    let movement = input / 1.5 * delta.delta_secs();
    transform.translation += movement;
    return Some(movement);
}

pub fn update_rigidbodies() {}
pub fn on_collision(
    trigger: Trigger<CollisionEvent>,
    mut rigidbody: Query<(&ShayminRigidbody, &mut Transform)>,
    other_col: Query<&Collider>,
) {
    let (rigidbody, transform) = rigidbody.get_mut(trigger.this).unwrap();
    let collider = other_col.get(trigger.other).unwrap();

    match &collider.shape {
        ColliderShape::Rect(rect) => todo!(),
        ColliderShape::Circle { radius } => todo!(),
        ColliderShape::Mesh(handle) => todo!(),
    }
}
