#![feature(int_roundings)]
#![feature(generic_arg_infer)]

use bevy::color::palettes::tailwind::{PINK_100, RED_500};
use bevy::prelude::*;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::RemotePlugin;
use bevy_editor_cam::prelude::*;
use bevy_picking::pointer::PointerInteraction;
use short_flight::ldtk::{self, LdtkMapBundle, SpawnMeshEvent};
use std::f32::consts::PI;

mod assets;
mod mesh;
mod npc;
mod player;

fn main() {
    App::new()
        // builtin
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        // game
        .add_plugins(assets::AssetsPlugin)
        .add_plugins(npc::NPCPlugin)
        .add_plugins(player::ShayminPlugin)
        // lib
        .add_plugins(short_flight::collision::CollisionPlugin)
        .add_plugins(short_flight::ldtk::LdtkPlugin)
        .add_plugins(mesh::TileMeshManagerPlugin)
        // third party
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        .add_plugins(bevy_sprite3d::Sprite3dPlugin)
        .add_plugins(bevy_editor_cam::DefaultEditorCamPlugins)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        // core game
        .add_systems(PreStartup, setup)
        .add_systems(Update, deferred_mesh_spawn)
        .add_systems(
            Update,
            (draw_mesh_intersections, pause_on_space, toggle_projection),
        )
        .add_event::<SpawnMeshEvent>()
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
            rotation: Quat::from_rotation_x(PI / -1.8),
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

fn deferred_mesh_spawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<&ldtk::LdtkMapHandle>,
) {
    let handle = asset_server
        .get_handle("tilemap.ldtk")
        .unwrap_or_else(|| asset_server.load::<ldtk::LdtkMap>("tilemap.ldtk"));
    for item in query.iter() {
        if item.0 == handle {
            return;
        }
    }
    commands.spawn((
        LdtkMapBundle {
            ldtk_map: ldtk::LdtkMapHandle(handle),
            ldtk_map_config: ldtk::LdtkMapConfig { selected_level: 0 },
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            global_transform: GlobalTransform::default(),
        },
        Name::new("LdtkMap"),
    ));
}

// lock the camera in place when space is held
fn pause_on_space(mut camera: Query<&mut EditorCam>, kb: Res<ButtonInput<KeyCode>>) {
    camera
        .iter_mut()
        .for_each(|mut camera| camera.enabled_motion.pan = !kb.pressed(KeyCode::Space));
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}
