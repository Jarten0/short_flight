use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

pub struct ShayminPlugin;

impl Plugin for ShayminPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shaymin)
            .add_systems(Update, control_shaymin);
    }
}

/// The protagonist/main player of the game.
#[derive(Debug, Component, Reflect, Clone)]
pub struct Shaymin {}

/// Init func for player code
pub fn spawn_shaymin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Shaymin {},
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE / 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.6, 0.25))),
        Transform::from_xyz(0.0, 1.5, 0.0),
    ));
}

pub fn control_shaymin(
    mut shaymin: Option<Single<(&mut Transform, &Shaymin), Without<Camera3d>>>,
    mut camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    let (mut transform, shaymin) = shaymin.unwrap().into_inner();
    let mut cam_transform = camera.unwrap().into_inner();
    transform.translation += {
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
        dir / 20.0
    };
    cam_transform.translation = {
        let mut vec3 = transform.translation;
        vec3.y += 10.0;
        vec3
    };
}
