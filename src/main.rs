#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(path_add_extension)]

use bevy::prelude::*;

pub(crate) mod animation;
pub(crate) mod collision;
pub(crate) mod editor;
pub(crate) mod sprite3d;

mod assets;
mod ldtk;
mod mesh;
mod moves;
mod npc;
mod player;
mod projectile;
mod tile;

fn main() {
    App::new()
        // builtin
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(MeshPickingPlugin)
        // game
        .add_plugins(assets::AssetsPlugin)
        .add_plugins(npc::NPCPlugin)
        .add_plugins(moves::interfaces::MovePlugin)
        .add_plugins(projectile::interfaces::ProjectilePlugin)
        .add_plugins(player::ShayminPlugin)
        .add_plugins(ldtk::LdtkPlugin)
        // lib
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(mesh::TileMeshManagerPlugin)
        .add_plugins(sprite3d::Sprite3dPlugin)
        // third party
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        .add_plugins(bevy_editor_cam::DefaultEditorCamPlugins)
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(bevy::remote::http::RemoteHttpPlugin::default())
        // core game
        .add_systems(PreStartup, setup)
        .add_systems(Update, toggle_projection)
        .run();
}

fn setup(mut commands: Commands) {
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
        // EditorCam::default().with_initial_anchor_depth(20.0),
    ));
}

fn toggle_projection(mut projection: Query<&mut Projection>, kb: Res<ButtonInput<KeyCode>>) {
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
