use anim_state::ShayminAnimation;
use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use bevy_sprite3d::prelude::*;
use short_flight::animation;

use crate::assets;
use crate::assets::shaymin::SpritesCollection;

mod anim_state;

pub struct ShayminPlugin;

impl Plugin for ShayminPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shaymin)
            .add_systems(
                Update,
                (
                    control_shaymin,
                    process_shaymin_collisions,
                    anim_state::update_materials,
                )
                    .chain(),
            )
            .add_plugins(Sprite3dPlugin);
    }
}

/// The protagonist/main player of the game.
#[derive(Debug, Component, Reflect, Clone)]
pub struct Shaymin {}

/// Init func for player code
fn spawn_shaymin(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    collection: Res<assets::shaymin::SpritesCollection>,
    sprite3d_params: Sprite3dParams,
) {
    let mesh = asset_server.add(Cuboid::from_size(Vec3::ONE / 2.0).into());
    let material = asset_server.add::<StandardMaterial>(Color::srgb(0.3, 0.6, 0.25).into());
    commands.spawn((
        Shaymin {},
        anim_state::animation(&asset_server),
        anim_state::sprite(&asset_server, collection, sprite3d_params),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(10.0, 1.5, -2.0),
    ));
}

fn control_shaymin(
    shaymin: Option<Single<(&mut Transform, &mut ShayminAnimation), Without<Camera3d>>>,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    delta: Res<Time<Fixed>>,
) {
    let (mut transform, mut anim) = shaymin.unwrap().into_inner();
    let mut cam_transform = camera.unwrap().into_inner();

    // match animation.current_animation {
    //     animation::AnimType::Idle => todo!(),
    //     animation::AnimType::Walking => todo!(),
    //     animation::AnimType::AttackSwipe => todo!(),
    //     animation::AnimType::AttackTackle => todo!(),
    //     animation::AnimType::Hurt => todo!(),
    //     animation::AnimType::Down => todo!(),
    // }

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

fn manage_movement(
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

fn process_shaymin_collisions(
    mut shaymin: Option<Single<(&mut Transform, &Shaymin), Without<Camera3d>>>,
    mut walls: Query<&TileStorage, Without<Camera3d>>,
) {
}
