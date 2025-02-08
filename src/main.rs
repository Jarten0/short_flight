use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::image::TextureFormatPixelInfo;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{
    Extent3d, Texture, TextureAspect, TextureFormat, TextureViewDescriptor,
};
use bevy_ecs_tilemap::prelude::*;
use bevy_editor_cam::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_picking::prelude::*;
use ldtk::{
    initialize_immediate_tilemaps, process_loaded_tile_maps, LdtkMap, LdtkMapBundle, LdtkMapHandle,
    LdtkPlugin,
};

pub mod ldtk;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(TilemapPlugin)
        .add_plugins(LdtkPlugin)
        .add_plugins(DefaultEditorCamPlugins)
        .add_systems(PreStartup, setup)
        .add_systems(Startup, spawn_protag)
        .add_systems(
            Update,
            (
                update_protag,
                spawn_mesh.after(process_loaded_tile_maps),
                draw_mesh_vertices,
            ),
        )
        .add_event::<SpawnMeshEvent>()
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // commands.spawn((Camera2d::default(), EditorCam::default()));
    commands.spawn((Camera3d::default(), EditorCam::default()));

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

#[derive(Debug, Event, Reflect, Clone)]
pub struct SpawnMeshEvent {
    tilemap: Entity,
}

fn draw_mesh_vertices(
    mut events: EventReader<SpawnMeshEvent>,
    mut commands: Commands,
    mut gizmos: Gizmos,
    asset_server: Res<AssetServer>,
    tilemaps: Query<&TileStorage>,
    query: Query<&TilePos>,
) {
    let mut vertices: Vec<[f32; 3]> = vec![];
    for tilemap in tilemaps.iter() {
        for (index, entity) in tilemap.iter().enumerate().filter(|item| item.1.is_some()) {
            let tile_pos = query.get(entity.unwrap()).unwrap();
            let x_pos = tile_pos.x as f32;
            let y_pos = tile_pos.y as f32;
            vertices.push([x_pos, 0.0, y_pos]);
            vertices.push([x_pos, 0.0, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, 0.0, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, 0.0, y_pos]);
        }
    }
    for vertex in vertices {
        gizmos.circle(Vec3::from(vertex), 0.04, palettes::basic::PURPLE);
    }
}

fn spawn_mesh(
    mut events: EventReader<SpawnMeshEvent>,
    mut commands: Commands,
    mut asset_server: Res<AssetServer>,
    maps: Res<Assets<LdtkMap>>,
    images: Res<Assets<Image>>,
    map_asset: Single<&LdtkMapHandle>,
    tilemaps: Query<(&TileStorage, &TilemapTexture)>,
    tiles: Query<(&TilePos, &TileTextureIndex)>,
) {
    for event in events.read() {
        log::info!("Spawning new mesh");

        let mut mapped_vertices_to_textures: HashMap<
            u32,
            (Vec<[f32; 3]>, Vec<u32>, Vec<[f32; 2]>),
        > = HashMap::new();

        let (tilemap_storage, tilemap_texture) = tilemaps.get(event.tilemap).unwrap();

        for (index, entity) in tilemap_storage
            .iter()
            .enumerate()
            .filter(|item| item.1.is_some())
        {
            let (tile_pos, tile_texture) = tiles.get(entity.unwrap()).unwrap();
            let x_pos = tile_pos.x as f32;
            let y_pos = tile_pos.y as f32;

            if !mapped_vertices_to_textures.contains_key(&tile_texture.0) {
                mapped_vertices_to_textures.insert(tile_texture.0, Default::default());
            }

            let (ref mut vertices, ref mut indices, ref mut texture_uvs) =
                mapped_vertices_to_textures
                    .get_mut(&tile_texture.0)
                    .unwrap();
            vertices.push([x_pos, 0.0, y_pos]);
            vertices.push([x_pos, 0.0, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, 0.0, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, 0.0, y_pos]);
            texture_uvs.push([0.0, 0.0]);
            texture_uvs.push([0.0, 1.0]);
            texture_uvs.push([1.0, 1.0]);
            texture_uvs.push([1.0, 0.0]);

            const FACE_INDICES: &[u32] = &[0, 1, 2, 0, 2, 3];

            let offseted = &mut FACE_INDICES
                .to_owned()
                .into_iter()
                .map(|value: u32| value + (index as u32 * 4))
                .collect();
            indices.append(offseted);
        }

        let texture: Handle<Image> = tilemap_texture.image_handles().get(0).unwrap().clone_weak();

        let tileset_texture: Image = images.get(&texture).unwrap().clone();
        let mut textures: Vec<Handle<Image>> = Vec::default();

        // // let floor =
        // //     |tile_width, image_width| (image_width as f32 / tile_width as f32).floor() as usize;

        let tile_width = 32;
        let tile_height = 32;
        let width = tileset_texture.width() as usize;
        let height = tileset_texture.height() as usize;

        assert!(
            !tileset_texture.is_compressed(),
            "cannot effectively resize compressed textures"
        );

        let bytes_per_pixel = tileset_texture.data.len() / (width * height);
        assert_eq!(
            bytes_per_pixel,
            tileset_texture.texture_descriptor.format.pixel_size()
        );

        // split up the image into a list of pixel lines
        let lines: Vec<&[u8]> = tileset_texture
            .data
            .chunks_exact(width * bytes_per_pixel)
            .collect();
        // bundle up each set of lines
        let chunked_lines = lines.chunks_exact(tile_height);
        // iterate over each bundle of lines and get x..x+width bytes of each line
        for x in 0..(width / tile_width) {
            let new_data: Vec<u8> = chunked_lines
                .clone()
                .flat_map(|chunk| {
                    chunk.iter().map(|line| {
                        (line[x * tile_width * bytes_per_pixel
                            ..(x + 1) * tile_width * bytes_per_pixel])
                            .to_owned()
                    })
                })
                .flatten()
                .collect();

            let new_image = Image::new(
                Extent3d {
                    width: tile_width as u32,
                    height: tile_height as u32,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                new_data,
                tileset_texture.texture_descriptor.format.clone(),
                tileset_texture.asset_usage.clone(),
            );
            // assert_eq!(new_image.data.len(), width * height * bytes_per_pixel);
            textures.push(asset_server.add(new_image));
        }

        // for tile_y in 0..floor(tile_height, height) {
        //     for tile_x in 0..floor(tile_width, width) {
        //         let new_image_data: Vec<u8> = (0..tile_height)
        //             .into_iter()
        //             .flat_map(|y| {
        //                 let start_index = tile_x + tile_y * width;
        //                 let y_offset = width * y;
        //                 let range = (start_index + y_offset) * bytes_per_pixel
        //                     ..(start_index + tile_width + y_offset) * bytes_per_pixel;
        //                 &tileset_texture.data[range]
        //             })
        //             .map(ToOwned::to_owned)
        //             .collect();
        //         let new_image = Image::new(
        //             Extent3d {
        //                 width: tile_width as u32,
        //                 height: tile_height as u32,
        //                 depth_or_array_layers: 1,
        //             },
        //             bevy::render::render_resource::TextureDimension::D2,
        //             new_image_data,
        //             tileset_texture.texture_descriptor.format.clone(),
        //             tileset_texture.asset_usage.clone(),
        //         );
        //         textures.push(asset_server.add(new_image));
        //     }
        // }
        // log::info!("finished {}x{} = {}", width, height, textures.len());

        for (texture_index, (vertices, indices, texture_uvs)) in mapped_vertices_to_textures {
            let image = textures[texture_index as usize].clone();

            log::info!("spawning mesh with {} vertices, {} indices, {} texture uvs, and this texture: {:?} from index {}", vertices.len(), indices.len(), texture_uvs.len(), &image, texture_index);

            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_inserted_indices(Indices::U32(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs);

            let material = StandardMaterial::from(image);

            commands.spawn((
                Mesh3d(asset_server.add(mesh)),
                MeshMaterial3d(asset_server.add(material)),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
    }
}

// wouldve named it 3DMode, but thats not allowed ig
#[derive(Debug, Default, Clone, Resource, Reflect)]
pub struct Mode3D(pub f32);
