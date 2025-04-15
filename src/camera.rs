use bevy::prelude::*;
use bevy::render::camera::CameraProjection;

pub struct CustomCameraPlugin;

#[derive(Debug, Deref, DerefMut, Default, Resource)]
pub struct Mode3D(f32);

impl Plugin for CustomCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup)
            .add_systems(Update, switch_projection);
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
        // Projection::custom(),
    ));
}

pub(crate) fn switch_projection(
    mut projection: Single<&mut Projection>,
    mode: Res<Mode3D>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    **projection = match **mode {
        0.0 => Projection::Orthographic(OrthographicProjection {
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
        0.0..1.0 => {
            todo!()
        }
        1.0 => Projection::Perspective(PerspectiveProjection {
            fov: core::f32::consts::PI / 4.0,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 16.0 / 9.0,
        }),
        mode => panic!("Invalid Mode3D value! [{}]", mode),
    }
}

#[derive(Debug, Component)]
pub struct OrthographicPerspectiveLerpProjection {
    base_perspective: PerspectiveProjection,
    base_orthographic: OrthographicProjection,
    t: f32,
}

impl Default for OrthographicPerspectiveLerpProjection {
    fn default() -> Self {
        Self {
            base_perspective: Default::default(),
            base_orthographic: OrthographicProjection::default_3d(),
            t: 0.0f32,
        }
    }
}

impl CameraProjection for OrthographicPerspectiveLerpProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        let mat = self.base_perspective.get_clip_from_view();
        let mat2 = self.base_orthographic.get_clip_from_view();
        (mat * self.t) + (mat2 * (1.0 - self.t))
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &bevy::render::camera::SubCameraView) -> Mat4 {
        let mat = self.base_perspective.get_clip_from_view_for_sub(sub_view);
        let mat2 = self.base_orthographic.get_clip_from_view_for_sub(sub_view);
        (mat * self.t) + (mat2 * (1.0 - self.t))
    }

    fn update(&mut self, width: f32, height: f32) {
        self.base_perspective.update(width, height);
        self.base_orthographic.update(width, height);
    }

    fn far(&self) -> f32 {
        f32::max(self.base_perspective.far(), self.base_orthographic.far())
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [bevy::math::Vec3A; 8] {
        self.base_perspective.get_frustum_corners(z_near, z_far)
    }
}
