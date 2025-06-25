use super::{Client, ClientQuery};
use crate::animation::{self, AnimType, cardinal};
use crate::collision::physics::Rigidbody;
use crate::collision::{
    self, BasicCollider, ColliderShape, CollisionEnterEvent, CollisionExitEvent, CollisionLayers,
    DynamicCollision, StaticCollision, TilemapCollision, ZHitbox,
};
use crate::ldtk::TileQuery;
use crate::moves::Move;
use crate::moves::interfaces::{MoveList, SpawnMove};
use crate::npc::animation::AnimationHandler;
use crate::npc::stats::FacingDirection;
use crate::tile::{TileDepth, TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::platform::collections::HashMap;
use bevy::platform::collections::HashSet;
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
        Rigidbody {
            ground: HashSet::default(),
            wall: HashSet::new(),
            velocity: Vec3::default(),
            previous_position: Vec3::default(),
            last_push: Vec3::default(),
        },
        DynamicCollision::default(),
        TilemapCollision,
    ));
    commands
        .entity(*shaymin)
        .observe(collision::physics::move_out_from_tilemaps)
        .observe(collision::physics::unground_on_leave)
        // .observe(
        //     |trigger: Trigger<CollisionEnterEvent>, mut query: Query<&mut Rigidbody>| {
        //         let Ok(mut rigidbody) = query.get_mut(trigger.this) else {
        //             return;
        //         };
        //         rigidbody.grounded = Some(trigger.other);
        //     },
        // )
        // .observe(
        //     |trigger: Trigger<CollisionExitEvent>, mut query: Query<&mut Rigidbody>| {
        //         let Ok(mut rigidbody) = query.get_mut(trigger.this) else {
        //             return;
        //         };
        //         rigidbody.grounded = None;
        //     },
        // )
        ;
}

pub fn control_shaymin(
    shaymin_entity: Client,
    shaymin: ClientQuery<
        (
            &Transform,
            &mut Rigidbody,
            Option<&mut AnimationHandler>,
            &mut FacingDirection,
        ),
        Without<Camera3d>,
    >,
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    move_list: Option<Res<MoveList>>,
    mut commands: Commands,
    mut gizmos: Gizmos,
) {
    let enable_gizmos: bool = kb.pressed(KeyCode::KeyX);
    let (transform, mut rigidbody, anim, mut facing) = shaymin.into_inner();

    if enable_gizmos {
        gizmos.arrow(
            transform.translation,
            transform.translation + Vec3::from((***facing, 1.0)).xzy(),
            palettes::basic::RED,
        );
    }

    let Some(mut anim) = anim else {
        return;
    };

    if let Some(data) = anim.animation_data()
        && let Some(move_list) = move_list
    {
        let input = get_input(&kb).normalize_or_zero();
        let input_dir = Dir2::new(input.xz().normalize_or(Vec2::NEG_Y)).unwrap();

        let mut rhs = *input_dir;
        if **facing == -input_dir {
            rhs -= Vec2::NEG_ONE;
        }

        let target = facing
            .move_towards(
                rhs,
                time.delta_secs() * facing.distance_squared(*input_dir) * 15.,
            )
            .normalize_or_zero();

        if input != Vec3::ZERO {
            facing.set(Dir2::new(target).unwrap_or(*FacingDirection::default()));
        }

        if enable_gizmos {
            gizmos.arrow(
                transform.translation,
                transform.translation + Vec3::from((*input_dir, 1.0)).xzy(),
                palettes::basic::GREEN,
            );
            // gizmos.arrow(
            //     transform.translation,
            //     transform.translation + Vec3::from((target, 1.0)).xzy(),
            //     palettes::basic::YELLOW,
            // );
        }

        if data.is_blocking() {
            return;
        }

        let delta_movement = time.delta_secs() * 30.;
        const MOVEBINDS: [(KeyCode, Move); 2] = [
            (KeyCode::KeyK, Move::MagicalLeaf),
            (KeyCode::KeyO, Move::Tackle),
        ];
        for (key, move_id) in MOVEBINDS {
            if kb.pressed(key) {
                use_move(
                    commands,
                    &mut rigidbody,
                    &mut anim,
                    delta_movement,
                    move_id,
                    &move_list,
                    *shaymin_entity,
                );
                return;
            };
        }
        rigidbody.velocity = rigidbody
            .velocity
            .xz()
            .move_towards(input.xz() * 1.5, delta_movement)
            .xxy()
            .with_y(rigidbody.velocity.y);
        if input.length_squared() <= 0.0 {
            anim.start_animation(AnimType::Idle);
            anim.looping = false;
            return;
        }

        let new_cardinal =
            cardinal(input_dir) != cardinal(**facing) || anim.current() != AnimType::Walking;

        if new_cardinal {
            anim.start_animation(animation::AnimType::Walking);
            anim.looping = true;
        } else if rigidbody.velocity == Vec3::ZERO {
            anim.start_animation(animation::AnimType::Idle);
        } else {
            anim.looping = true;
        }
    }
}

fn use_move(
    mut commands: Commands,
    rigidbody: &mut Mut<Rigidbody>,
    anim: &mut Mut<AnimationHandler>,
    delta_movement: f32,
    move_id: Move,
    move_list: &MoveList,
    parent: Entity,
) {
    let data = move_list.data.get(&move_id).unwrap();
    anim.start_animation(animation::AnimType::AttackShoot);
    commands.queue(SpawnMove { move_id, parent });
    // rigidbody.velocity = rigidbody
    //     .velocity
    //     .xz()
    //     .move_towards(Vec2::ZERO, delta_movement * 2.)
    //     .xxy()
    //     .with_y(rigidbody.velocity.y);
}

pub fn get_input(kb: &ButtonInput<KeyCode>) -> Vec3 {
    let mut dir: Vec3 = Vec3::ZERO;

    if kb.pressed(KeyCode::ShiftLeft) {
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
        AnyOf<(&Rigidbody, &StaticCollision, &DynamicCollision)>,
        &GlobalTransform,
        &ZHitbox,
    )>,
    transform_query: Query<&GlobalTransform>,
    shaymin: Client,
    mut gizmos: Gizmos,
) {
    if let Ok((collider, (dyn_info, stat_info, _dyn_info), transform, zhitbox)) =
        rigidbodies.get(shaymin.entity())
    {
        for entity in &collider.currently_colliding {
            gizmos
                .sphere(
                    Isometry3d::new(
                        transform_query
                            .get(*entity)
                            .unwrap()
                            .translation()
                            .with_y(5.0),
                        Quat::from_rotation_x(f32::to_radians(-90.0)),
                    ),
                    0.20,
                    palettes::basic::NAVY,
                )
                .resolution(4);
        }
    }

    for (collider, (dyn_info, stat_info, _dyn_info), transform, zhitbox) in &rigidbodies {
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
        if let Some(dyn_info) = dyn_info
            && dyn_info.last_push.normalize_or(Vec3::NAN) == Vec3::NAN
        {
            gizmos.arrow(
                translation + (Vec3::Y * 8.),
                translation + (Vec3::Y * 8.) + dyn_info.last_push * 8.,
                palettes::basic::LIME,
            );
        }
    }
}

pub fn draw_tile_collision(
    mut gizmos: Gizmos,
    tile_query: TileQuery,
    tile_data: Query<(&Transform, &TileDepth, &TileSlope, &TileFlags)>,
) {
    let Some((transform, depth, slope, flags)) = tile_query
        .get_tile(Vec3::ZERO)
        .map(|tile| tile_data.get(tile).ok())
        .unwrap_or(None)
    else {
        return;
    };
}
