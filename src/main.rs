#![feature(int_roundings)]

use bevy::color::palettes::tailwind::{PINK_100, RED_500};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_editor_cam::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_picking::pointer::PointerInteraction;
use short_flight::ldtk;
use short_flight::ldtk::{LdtkMapBundle, LdtkPlugin, SpawnMeshEvent};
use std::f32::consts::PI;

mod mesh;
mod player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(player::ShayminPlugin)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(TilemapPlugin)
        .add_plugins(LdtkPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(DefaultEditorCamPlugins)
        .add_systems(PreStartup, setup)
        .add_systems(
            Update,
            (
                mesh::spawn_mesh.after(ldtk::process_loaded_tile_maps),
                (
                    mesh::call_save_event,
                    |mut commands: Commands, mut event_reader: EventReader<mesh::SaveEvent>| {
                        for event in event_reader.read() {
                            commands.trigger(*event);
                        }
                    },
                )
                    .chain(),
                draw_mesh_intersections,
                pause_on_space,
            ),
        )
        .add_event::<SpawnMeshEvent>()
        .add_event::<mesh::SaveEvent>()
        .add_observer(mesh::save_tile_data)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(PI / -2.0),
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
        EditorCam::default().with_initial_anchor_depth(20.0),
    ));

    let tilemap = asset_server.load("tilemap.ldtk");

    commands.spawn((
        LdtkMapBundle {
            ldtk_map: ldtk::LdtkMapHandle(tilemap),
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
