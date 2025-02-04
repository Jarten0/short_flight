use std::ops::Deref;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology};
use bevy_ecs_tilemap::prelude::*;
use bevy_editor_cam::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_picking::prelude::*;
use ldtk::{LdtkMap, LdtkMapBundle, LdtkPlugin};

pub mod ldtk;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(TilemapPlugin)
        .add_plugins(LdtkPlugin)
        .add_plugins(DefaultEditorCamPlugins)
        .add_systems(Startup, (setup, spawn_mesh.after(setup), spawn_protag))
        .add_systems(Update, update_protag)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    log::info!("PreStartup");
    commands.spawn((Camera3d::default(), EditorCam::default()));

    commands.spawn((
        LdtkMapBundle {
            ldtk_map: ldtk::LdtkMapHandle(asset_server.load("tilemap.ldtk")),
            ldtk_map_config: ldtk::LdtkMapConfig { selected_level: 0 },
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            global_transform: GlobalTransform::default(),
        },
        Name::new("LdtkMap"),
    ));
}

#[derive(Debug, Default, Component, Reflect, Clone)]
pub struct Protag {}

fn spawn_protag(mut commands: Commands) {
    commands.spawn((
        Protag::default(),
        Transform::default(),
        Sprite::from_color(LinearRgba::RED, Vec2::ONE * 40.0),
        Name::new("Protag"),
    ));
}

fn update_protag(kb: Res<ButtonInput<KeyCode>>, mut protag: Single<(&mut Transform, &Protag)>) {
    for (input, dir) in &[
        (KeyCode::KeyD, Vec3::X),
        (KeyCode::KeyA, Vec3::NEG_X),
        (KeyCode::KeyS, Vec3::NEG_Y),
        (KeyCode::KeyW, Vec3::Y),
    ] {
        if kb.pressed(*input) {
            protag.0.translation += dir * 5.;
        }
    }
}

const CONSTA: f32 = 40.0;
fn spawn_mesh(
    tilemaps: Query<&TileStorage>,
    query: Query<&TilePos>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    log::info!("PostStartup");

    let mut vertices: Vec<[f32; 3]> = vec![];
    let mut indices: Vec<u32> = vec![];

    assert!(tilemaps.iter().len() > 0);

    for tilemap in tilemaps.iter() {
        for (index, entity) in tilemap.iter().enumerate().filter(|item| item.1.is_some()) {
            let tile_pos = query.get(entity.unwrap()).unwrap();
            let x_pos = tile_pos.x as f32;
            let y_pos = tile_pos.y as f32;
            vertices.push([x_pos, y_pos, 0.0]);
            vertices.push([x_pos, y_pos + 1.0, 0.0]);
            vertices.push([x_pos + 1.0, y_pos + 1.0, 0.0]);
            vertices.push([x_pos + 1.0, y_pos, 0.0]);

            const FACE_INDICES: &[u32] = &[0, 1, 2, 0, 2, 3];

            let offseted = &mut FACE_INDICES
                .to_owned()
                .into_iter()
                .map(|value: u32| value + (index as u32 * 4))
                .collect();
            indices.append(offseted);
        }
    }

    dbg!(&vertices);
    dbg!(&indices);

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(Indices::U32(indices));

    let mesh_handle = asset_server.add(mesh);
    let material_handle = asset_server.add(StandardMaterial::from_color(LinearRgba::GREEN));

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));
}

// wouldve named it 3DMode, but thats not allowed ig
#[derive(Debug, Default, Clone, Resource, Reflect)]
pub struct Mode3D(pub f32);
