use crate::ldtk::{self, SpawnMeshEvent, TileDepth};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use bevy_ecs_tilemap::prelude::*;
use image::ImageBuffer;
use serde::Serialize;
use short_flight::ldtk::TileSlope;
use short_flight::serialize_to_file;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

pub struct TileMeshManagerPlugin;

impl Plugin for TileMeshManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_mesh.after(ldtk::process_loaded_tile_maps),
                (
                    call_save_event,
                    |mut commands: Commands, mut event_reader: EventReader<SaveEvent>| {
                        for event in event_reader.read() {
                            commands.trigger(*event);
                        }
                    },
                )
                    .chain(),
                // manage_tile_refresh_events,
            ),
        )
        .add_event::<SaveEvent>()
        // .add_observer(|trigger: Trigger<>|)
        .add_observer(save_tile_data);
    }
}

fn spawn_mesh(
    mut events: EventReader<SpawnMeshEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    tilemaps: Query<(&TileStorage, &TilemapTexture)>,
    tiles: Query<(&TilePos, Option<&TileTextureIndex>, &TileDepth)>,
) {
    for event in events.read() {
        log::info!("Spawning new mesh");

        let mut mesh_information: HashMap<
            Entity,
            (
                Vec<[f32; 3]>,
                Vec<u32>,
                Vec<[f32; 2]>,
                Vec<[f32; 3]>,
                Option<usize>,
                Option<&TileDepth>,
            ),
        > = HashMap::new();

        let (tilemap_storage, tilemap_texture) = tilemaps.get(event.tilemap).unwrap();

        for entity in tilemap_storage.iter().filter_map(|item| *item) {
            let (tile_pos, tile_texture, tile_depth) = tiles.get(entity).unwrap();
            let x_pos = tile_pos.x as f32;
            let y_pos = -(tile_pos.y as f32);

            mesh_information.insert(entity, Default::default());

            // pull a reference instead of declaring so that changing type declaration is easier
            let (
                ref mut vertices,
                ref mut indices,
                ref mut texture_uvs,
                ref mut normals,
                ref mut texture_index,
                ref mut tiledepth,
            ) = mesh_information.get_mut(&entity).unwrap();

            const FACE_INDICES: &[u32] = &[
                0, 1, 2, 0, 2, 3, 0, 3, 7, 0, 7, 4, 1, 0, 4, 1, 4, 5, 2, 1, 5, 2, 5, 6, 3, 2, 6, 3,
                6, 7,
            ];
            let vertex_data = get_vertex_data(x_pos, y_pos);
            *vertices = vertex_data.clone().into_iter().map(|(v, _, _)| v).collect();
            *texture_uvs = vertex_data
                .clone()
                .into_iter()
                .map(|(_, uv, _)| uv)
                .collect();
            *normals = vertex_data.clone().into_iter().map(|(_, _, n)| n).collect();
            indices.append(&mut FACE_INDICES.to_vec());
            tile_texture.map(|texture| texture_index.insert(texture.0 as usize));
            let _ = tiledepth.insert(tile_depth);
        }

        let image_handles = tilemap_texture.image_handles();
        let Some(texture) = image_handles.get(0) else {
            log::error!("Could not find the tilemap texture!");
            continue;
        };

        let Some(get) = images.get(*texture) else {
            log::error!("Could not find tilemap texture {:?}!", texture);
            continue;
        };

        let tileset_texture: Image = get.clone();
        let mut textures: Vec<Handle<Image>> = Vec::default();

        let tile_width = 32;
        let tile_height = 32;
        let image_width = tileset_texture.width();
        let image_height = tileset_texture.height();

        let x = image_width.div_floor(tile_width);
        let y = image_height.div_floor(tile_height);

        let base_image = image::DynamicImage::ImageRgba8(
            ImageBuffer::from_raw(image_width, image_height, tileset_texture.data).unwrap(),
        );

        for y in 0..y {
            for x in 0..x {
                let new_image = Image::from_dynamic(
                    base_image.crop_imm(x * tile_width, y * tile_height, tile_width, tile_height),
                    true,
                    RenderAssetUsages::RENDER_WORLD,
                );

                textures.push(asset_server.add(new_image));
            }
        }

        log::info!("Spliced {} images from {:?}", textures.len(), texture);

        let texture_materials: Vec<Handle<StandardMaterial>> = textures
            .into_iter()
            .map(|image_handle| asset_server.add(StandardMaterial::from(image_handle.clone())))
            .collect();
        let hovering: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::GREEN));
        let selected: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::LIME));
        let missing: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::PURPLE));

        let mut mesh_bundle_inserts = HashMap::new();

        for (entity, (vertices, indices, texture_uvs, normals, texture_index, tile_depth)) in
            mesh_information
        {
            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_indices(Indices::U32(indices));

            let material = texture_index
                .map(|texture_index| texture_materials.get(texture_index).unwrap().clone())
                .unwrap_or(missing.clone());

            commands
                .entity(entity)
                .observe(update_material_on::<Pointer<Over>>(hovering.clone()))
                .observe(update_material_on::<Pointer<Out>>(material.clone()))
                .observe(update_material_on::<Pointer<Down>>(selected.clone()))
                .observe(update_material_on::<Pointer<Up>>(hovering.clone()))
                .observe(move_on_drag)
                .observe(set_tile_slope)
                .observe(set_tile_depth_out)
                .observe(set_tile_depth_up);

            mesh_bundle_inserts.insert(
                entity,
                (
                    Mesh3d(asset_server.add(mesh)),
                    MeshMaterial3d(material),
                    Transform::from_xyz(
                        0.0,
                        tile_depth.map(|t| t.0).unwrap_or_default() as f32,
                        0.0,
                    ),
                ),
            );
        }

        commands.insert_batch(mesh_bundle_inserts);
    }
}

fn get_vertex_data(
    x_pos: f32,
    y_pos: f32,
    // slope_pos: IVec2,
    // referenced_slope_height: i64,
) -> Vec<([f32; 3], [f32; 2], [f32; 3])> {
    // let slope = Vec2::from(slope_pos);
    // let z_pos = [
    //     slope.x / referenced_slope_height as f32
    // ];
    vec![
        ([x_pos, 0.0, y_pos], [0.0, 0.0], [0.0, 1.0, 0.0]),
        ([x_pos, 0.0, y_pos + 1.0], [0.0, 1.0], [0.0, 1.0, 0.0]),
        ([x_pos + 1.0, 0.0, y_pos + 1.0], [1.0, 1.0], [0.0, 1.0, 0.0]),
        ([x_pos + 1.0, 0.0, y_pos], [1.0, 0.0], [0.0, 1.0, 0.0]),
        ([x_pos, -5.0, y_pos], [0.0, 0.0], [0.0, 1.0, 0.0]),
        ([x_pos, -5.0, y_pos + 1.0], [0.0, 1.0], [0.0, 1.0, 0.0]),
        (
            [x_pos + 1.0, -5.0, y_pos + 1.0],
            [1.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        ([x_pos + 1.0, -5.0, y_pos], [1.0, 0.0], [0.0, 1.0, 0.0]),
    ]
}

/// Returns an observer that updates the entity's material to the one specified.
fn update_material_on<E>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(Trigger<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    // An observer closure that captures `new_material`. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // material. Instead, the event type is a generic, and the material is passed in.
    move |trigger, mut query| {
        if let Ok(mut material) = query.get_mut(trigger.entity()) {
            material.0 = new_material.clone();
        }
    }
}

fn move_on_drag(
    drag: Trigger<Pointer<Drag>>,
    mut transforms: Query<(&mut Transform)>,
    // mut tile_change_writer: EventWriter<TileChangeTracker>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if kb.pressed(KeyCode::ShiftLeft) {
        return;
    }
    let (mut transform) = transforms.get_mut(drag.entity()).unwrap();
    transform.translation.y -= drag.delta.y * 0.01;
    // tile_change_writer.send(tile_change.clone());
}

fn set_tile_slope(
    drag: Trigger<Pointer<Down>>,
    mut slopes: Query<&mut TileSlope>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if !kb.pressed(KeyCode::ShiftLeft) {
        return;
    }
    let mut slope = slopes.get_mut(drag.entity()).unwrap();
    if kb.just_pressed(KeyCode::KeyA) {
        slope.0.x -= 1;
    }
    if kb.just_pressed(KeyCode::KeyD) {
        slope.0.x += 1;
    }
    if kb.just_pressed(KeyCode::KeyW) {
        slope.0.y += 1;
    }
    if kb.just_pressed(KeyCode::KeyS) {
        slope.0.y -= 1;
    }
}

fn set_tile_depth_up(
    drag: Trigger<Pointer<DragEnd>>,
    transforms: Query<(&mut TileDepth, &mut Transform)>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    set_tile_depth(drag.entity(), transforms);
}

fn set_tile_depth_out(
    drag: Trigger<Pointer<Out>>,
    transforms: Query<(&mut TileDepth, &mut Transform)>,
) {
    set_tile_depth(drag.entity(), transforms);
}

fn set_tile_depth(entity: Entity, mut transforms: Query<(&mut TileDepth, &mut Transform)>) {
    let (mut depth, mut transform) = transforms.get_mut(entity).unwrap();
    transform.translation.y = transform.translation.y.floor();
    depth.0 = transform.translation.y as i64;
}

#[derive(Debug, Clone, Copy, Event, Reflect)]
struct SaveEvent;

fn save_tile_data(
    _save: Trigger<SaveEvent>,
    tilemaps: Query<&TileStorage>,
    tiles: Query<(&TilePos, &TileDepth, &TileSlope)>,
) {
    let mut depth_info: ldtk::TileDepthMapSerialization = HashMap::new();
    let mut slope_info: ldtk::TileSlopeMapSerialization = HashMap::new();
    for tilemap in tilemaps.iter() {
        tilemap
            .iter()
            .filter_map(|item| item.map(|item| tiles.get(item).ok()).unwrap_or_default())
            .for_each(|(pos, depth, slope)| {
                depth_info.insert([pos.x, pos.y], depth.0);
                slope_info.insert([pos.x, pos.y], slope.0);
            });
    }

    let path = "assets/depth_maps/tile_depth_map.ron";
    if serialize_to_file(depth_info, path) {
        log::info!("Saved tile depth map!")
    }
    let path = "assets/depth_maps/tile_slope_map.ron";
    if serialize_to_file(slope_info, path) {
        log::info!("Saved tile slope map!")
    }
}

fn call_save_event(kb: Res<ButtonInput<KeyCode>>, mut saves: EventWriter<SaveEvent>) {
    if kb.just_pressed(KeyCode::KeyI) {
        log::info!("Saving tile depth map...");
        saves.send(SaveEvent);
    }
}

// fn manage_tile_refresh_events(
//     mut tile_changes: EventReader<TileChangeTracker>,
//     mut commands: Commands,
// ) {
//     let mut entities_to_refresh: HashMap<&Entity, Vec<TileChanged>> = HashMap::new();
//     for event in tile_changes.read() {
//         for entity in event.0.iter() {
//             if let Some(events) = entities_to_refresh.get_mut(entity) {
//                 events.push(event);
//             } else {
//                 entities_to_refresh.insert(entity, vec![event]);
//             }
//         }
//     }
//     for (entity, events) in entities_to_refresh {
//         commands.trigger();
//     }
//     tile_changes.clear();
// }

// /// When this tile is changed, let all of these entities know
// #[derive(Debug, Clone, Event)]
// struct TileChangeTracker(Vec<Entity>);
// #[derive(Debug, Clone, Copy, Event)]
// struct TileChanged { slope: bool }

// fn update_tile_slope(mut tile: Query<&TileSlope, &mut Mesh3d>) {}
