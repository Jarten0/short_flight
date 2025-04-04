use super::anim_state::ShayminAnimation;
use super::{Client, ClientQuery};
use crate::ldtk::{TileDepth, TileFlags, TileSlope};
use crate::mesh;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;
use short_flight::animation;
use short_flight::collision::{
    Collider, ColliderShape, CollisionEvent, CollisionLayers, DynamicCollision, StaticCollision,
    ZHitbox,
};

#[derive(Debug, Component)]
#[require(DynamicCollision)]
pub struct ShayminRigidbody {
    previous_position: Vec3,
}

pub fn setup(shaymin: Client, mut commands: Commands) {
    commands.entity(*shaymin).insert((
        Collider {
            dynamic: true,
            shape: ColliderShape::Circle(20. / 32. / 2.),
            layers: CollisionLayers::NPC,
            can_interact: CollisionLayers::NPC
                | CollisionLayers::Projectile
                | CollisionLayers::Wall,
        },
        ZHitbox {
            y_tolerance: 0.5,
            neg_y_tolerance: 0.0,
        },
        ShayminRigidbody {
            previous_position: Vec3::ZERO,
        },
        DynamicCollision {},
    ));
    commands.add_observer(on_collision);
}

pub fn control_shaymin(
    shaymin: ClientQuery<(&mut Transform, Option<&mut ShayminAnimation>), Without<Camera3d>>,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    delta: Res<Time<Fixed>>,
) {
    let (mut transform, anim) = shaymin.into_inner();

    transform.translation.y -= 4. * delta.delta_secs();

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
        &Collider,
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
pub fn update_rigidbodies(mut rigidbodies: Query<(&mut ShayminRigidbody, &Transform)>) {
    for (mut rigidbody, transform) in &mut rigidbodies {
        if rigidbody.previous_position != transform.translation {
            rigidbody.previous_position = transform.translation;
        }
    }
}

pub fn on_collision(
    trigger: Trigger<CollisionEvent>,

    mut rigidbody: Query<(&ShayminRigidbody, &GlobalTransform, &mut Transform)>,
    other_col: Query<(
        &Collider,
        &GlobalTransform,
        &ZHitbox,
        Option<((&TilePos, &TileDepth, &TileSlope, &TileFlags))>,
    )>,
    mut gizmos: Gizmos,
) {
    let (rigidbody, global_transform, mut transform) = rigidbody.get_mut(trigger.this).unwrap();
    let global_translation = global_transform.translation();
    let relative_translation = &mut transform.translation;

    let (collider, global_transform2, zhitbox, tile_query) = other_col.get(trigger.other).unwrap();
    let global_translation2 = global_transform2.translation();

    let start = *relative_translation;

    if let Some((_tile_pos, tile_depth, tile_slope, _tile_flags)) = tile_query {
        let tile_pos = global_translation2;
        // let ColliderShape::Rect(rect) = &collider.shape else {
        //     return;
        // };
        let movement = rigidbody.previous_position - *relative_translation;

        // floor and roof hitboxes stretch to match player velocity :3
        let hitbox_height = tile_pos.y + movement.y + 0.3;

        // let slope = {
        //     mesh::get_slope_corner_depths(tile_slope, inclusive)
        // }

        if global_translation.y <= hitbox_height {
            let tile_top = tile_pos.y + tile_slope.0.length();
            relative_translation.y = tile_top;
        } else if global_translation.y >= tile_pos.y - hitbox_height + zhitbox.height() {
            relative_translation.y = tile_pos.y + tile_slope.0.length();
        } else {
            *relative_translation -= Vec3::new(movement.x, 0.0, movement.z) * 8.;
        }

        // log::info!("{}", (hitbox_height));
    };

    // match &collider.shape {
    //     ColliderShape::Rect(rect) => {}
    //     ColliderShape::Circle { radius } => {}
    //     ColliderShape::Mesh(handle) => todo!(),
    // }
}
