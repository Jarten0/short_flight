use bevy::prelude::*;

pub struct CustomCameraPlugin;

impl Plugin for CustomCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup)
            .add_systems(Update, toggle_projection);
    }
}

pub(crate) fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(core::f32::consts::PI / -1.8),
            ..default()
        },
        ShowLightGizmo::default(),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::default()
            .looking_at(Vec3::NEG_Y, Vec3::Y)
            .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0)))
            .with_translation(Vec3::new(0.0, 20.0, 0.0)),
    ));
}

pub(crate) fn toggle_projection(
    mut projection: Query<&mut Projection>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if kb.just_pressed(KeyCode::KeyT) {
        for mut proj in &mut projection {
            *proj = match &*proj {
                Projection::Perspective(_) => Projection::Orthographic(OrthographicProjection {
                    scale: 1.0,
                    near: 0.1,
                    far: 1000.0,
                    viewport_origin: Vec2::new(0.5, 0.5),
                    scaling_mode: bevy::render::camera::ScalingMode::AutoMax {
                        max_width: 16.,
                        max_height: 9.,
                    },
                    area: Rect::new(-1.0, -1.0, 1.0, 1.0),
                }),
                Projection::Orthographic(_) => Projection::Perspective(PerspectiveProjection {
                    fov: core::f32::consts::PI / 4.0,
                    near: 0.1,
                    far: 1000.0,
                    aspect_ratio: 16.0 / 9.0,
                }),
            }
        }
    }
}
