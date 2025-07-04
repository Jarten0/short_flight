use crate::billboard::{self, BillboardTarget};
use crate::shaymin::{Client, ClientQuery, Shaymin};
use bevy::prelude::*;
use bevy::render::camera::CameraProjection;

const OVERRIDE_PERSPECTIVE: Option<f32> =
    // Default
    None;
// Some(1.0);

const OVERRIDE_ORIENTATION: Option<f32> =
    // Default
    // None;
    Some(f32::to_radians(-90.0 * 5. / 6.));

const OVERRIDE_POSITION_OFFSET: Option<Vec3> =
    // Default
    // None;
    Some(Vec3::new(0.0, 20.0, 5.0));

static ORTHOGRAPHIC_PROJECTION: OrthographicProjection = OrthographicProjection {
    scale: 1.0,
    near: 0.1,
    far: 1000.0,
    viewport_origin: Vec2::new(0.5, 0.5),
    scaling_mode: bevy::render::camera::ScalingMode::AutoMax {
        max_width: 16.,
        max_height: 9.,
    },
    area: {
        let x0 = -1.0;
        let y0 = -1.0;
        {
            let p0 = Vec2::new(x0, y0);
            let p1 = Vec2::new(1.0, 1.0);
            Rect {
                min: Vec2 {
                    x: p0.x.min(p1.x),
                    y: p0.y.min(p1.y),
                },
                max: Vec2 {
                    x: p0.x.max(p1.x),
                    y: p0.y.max(p1.y),
                },
            }
        }
    },
};
static PERSPECTIVE_PROJECTION: PerspectiveProjection = PerspectiveProjection {
    fov: core::f32::consts::PI / 4.0,
    near: 0.1,
    far: 1000.0,
    aspect_ratio: 16.0 / 9.0,
};

pub struct CustomCameraPlugin;

#[derive(Debug, Deref, DerefMut, Default, Resource)]
pub struct Mode3D(f32);

impl Plugin for CustomCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mode3D>()
            .add_systems(PreStartup, setup)
            .add_systems(
                Update,
                (
                    switch_projection,
                    follow_player,
                    billboard::update_billboards,
                ),
            );
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

    let camera = commands
        .spawn((
            Camera3d::default(),
            Transform::default()
                .looking_at(Vec3::NEG_Y, Vec3::Y)
                .with_rotation(match OVERRIDE_ORIENTATION {
                    Some(some) => Quat::from_rotation_x(some),
                    None => Quat::from_rotation_x(f32::to_radians(-90.0)),
                })
                .with_translation(match OVERRIDE_POSITION_OFFSET {
                    Some(some) => some,
                    None => Vec3::new(0.0, 20.0, 0.0),
                }),
        ))
        .id();
    billboard::DEFAULT_BILLBOARD_TARGET
        .set(camera)
        .expect("Failed to initialize camera as default billboard target");
}

pub(crate) fn switch_projection(mut projection: Single<&mut Projection>, mode: Res<Mode3D>) {
    let mode3d = match OVERRIDE_PERSPECTIVE {
        Some(some) => some,
        None => **mode,
    };

    **projection = match mode3d {
        0.0 => Projection::Orthographic(ORTHOGRAPHIC_PROJECTION.clone()),
        0.0..1.0 => Projection::custom(OrthographicPerspectiveLerpProjection {
            base_perspective: PERSPECTIVE_PROJECTION.clone(),
            base_orthographic: ORTHOGRAPHIC_PROJECTION.clone(),
            s: mode3d,
        }),
        1.0 => Projection::Perspective(PERSPECTIVE_PROJECTION.clone()),
        mode => panic!("Invalid Mode3D value! [{}]", mode),
    }
}

pub(crate) fn follow_player(
    camera: Option<Single<&mut Transform, (With<Camera3d>, Without<Shaymin>)>>,
    transform: Option<ClientQuery<&Transform>>,
) {
    if let Some(transform) = transform {
        let mut cam_transform = camera.unwrap().into_inner();
        cam_transform.translation = transform.translation
            + match OVERRIDE_POSITION_OFFSET {
                Some(some) => some,
                None => Vec3::new(0.0, 20.0, 0.0),
            };
    };
}

#[derive(Debug, Component, Clone)]
pub struct OrthographicPerspectiveLerpProjection {
    base_perspective: PerspectiveProjection,
    base_orthographic: OrthographicProjection,
    s: f32,
}

impl Default for OrthographicPerspectiveLerpProjection {
    fn default() -> Self {
        Self {
            base_perspective: Default::default(),
            base_orthographic: OrthographicProjection::default_3d(),
            s: 0.0f32,
        }
    }
}

impl CameraProjection for OrthographicPerspectiveLerpProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        let mat = self.base_perspective.get_clip_from_view();
        let mat2 = self.base_orthographic.get_clip_from_view();
        (mat * self.s) + (mat2 * (1.0 - self.s))
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &bevy::render::camera::SubCameraView) -> Mat4 {
        let mat = self.base_perspective.get_clip_from_view_for_sub(sub_view);
        let mat2 = self.base_orthographic.get_clip_from_view_for_sub(sub_view);
        (mat * self.s) + (mat2 * (1.0 - self.s))
    }

    fn update(&mut self, width: f32, height: f32) {
        self.base_perspective.update(width, height);
        self.base_orthographic.update(width, height);
    }

    fn far(&self) -> f32 {
        f32::lerp(
            self.base_perspective.far(),
            self.base_orthographic.far(),
            self.s,
        )
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [bevy::math::Vec3A; 8] {
        let perspective = self.base_perspective.get_frustum_corners(z_near, z_far);
        let orthographic = self.base_orthographic.get_frustum_corners(z_near, z_far);
        let iter = perspective
            .into_iter()
            .zip(orthographic)
            .map(|(p, o)| Vec3A::from(Vec3::from(p).lerp(o.into(), self.s)));
        debug_assert!(
            orthographic
                .iter()
                .map(|value| value.is_finite())
                .fold(true, |a, b| a & b)
        );
        *iter.collect::<Vec<Vec3A>>().as_array().unwrap()
    }
}
