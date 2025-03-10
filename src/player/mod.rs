use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use short_flight::animation;

mod anim_state;

pub struct ShayminPlugin;

impl Plugin for ShayminPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shaymin).add_systems(
            Update,
            (
                control_shaymin,
                process_shaymin_collisions,
                anim_state::update_materials,
            )
                .chain(),
        );
    }
}

/// The protagonist/main player of the game.
#[derive(Debug, Component, Reflect, Clone)]
pub struct Shaymin {}

/// Init func for player code
fn spawn_shaymin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_server: ResMut<AssetServer>,
) {
    commands.spawn((
        Shaymin {},
        anim_state::ShayminAnimation::new(&mut asset_server),
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE / 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.6, 0.25))),
        Transform::from_xyz(10.0, 1.5, -2.0),
    ));
}

fn control_shaymin(
    shaymin: Option<Single<(&mut Transform, &anim_state::ShayminAnimation), Without<Camera3d>>>,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    delta: Res<Time<Fixed>>,
) {
    let (mut transform, anim) = shaymin.unwrap().into_inner();
    let mut cam_transform = camera.unwrap().into_inner();

    // match animation.current_animation {
    //     animation::AnimType::Idle => todo!(),
    //     animation::AnimType::Walking => todo!(),
    //     animation::AnimType::AttackSwipe => todo!(),
    //     animation::AnimType::AttackTackle => todo!(),
    //     animation::AnimType::Hurt => todo!(),
    //     animation::AnimType::Down => todo!(),
    // }

    if anim.pool[&anim.current].can_move() {
        if let Some(movement) = manage_movement(kb, &mut transform, &delta) {
            // if anim.pool[&anim.current].
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
