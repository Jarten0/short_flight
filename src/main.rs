#![feature(int_roundings)]
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::color::palettes::tailwind::{PINK_100, RED_500};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_ecs_tilemap::prelude::*;
use bevy_editor_cam::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_picking::pointer::PointerInteraction;
use bevy_picking::prelude::*;
use image::ImageBuffer;
use short_flight::ldtk::{
    self, initialize_immediate_tilemaps, process_loaded_tile_maps, LdtkMap, LdtkMapBundle,
    LdtkMapHandle, LdtkPlugin, SpawnMeshEvent, TileDepth,
};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;

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
                spawn_mesh.after(process_loaded_tile_maps),
                draw_mesh_intersections,
                pause_on_space,
                (
                    call_save_event,
                    |mut commands: Commands, mut event_reader: EventReader<SaveEvent>| {
                        for event in event_reader.read() {
                            commands.trigger(*event);
                        }
                    },
                )
                    .chain(),
            ),
        )
        .add_event::<SpawnMeshEvent>()
        .add_event::<SaveEvent>()
        .add_observer(save_tile_data)
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
            let vertex_data = [
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
            ];
            *vertices = vertex_data.into_iter().map(|(v, _, _)| v).collect();
            *texture_uvs = vertex_data.into_iter().map(|(_, uv, _)| uv).collect();
            *normals = vertex_data.into_iter().map(|(_, _, n)| n).collect();
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

/// An observer to rotate an entity when it is dragged
fn move_on_drag(
    drag: Trigger<Pointer<Drag>>,
    mut transforms: Query<&mut Transform>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let mut transform = transforms.get_mut(drag.entity()).unwrap();
    transform.translation.y -= drag.delta.y * 0.01;
    if kb.pressed(KeyCode::KeyR) {
        transform.translation.y = (transform.translation.y * 10.0).floor() / 10.0
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
    transform.translation.y = transform.translation.y.round();
    depth.0 = transform.translation.y as i64;
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

#[derive(Debug, Clone, Copy, Event, Reflect)]
pub struct SaveEvent;

fn save_tile_data(
    _save: Trigger<SaveEvent>,
    tilemaps: Query<&TileStorage>,
    tiles: Query<(&TilePos, &TileDepth)>,
) {
    let mut tile_info: ldtk::TileDepthMapSerialization = HashMap::new();
    for tilemap in tilemaps.iter() {
        tilemap
            .iter()
            .filter_map(|item| item.map(|item| tiles.get(item).ok()).unwrap_or_default())
            .for_each(|(pos, depth)| {
                tile_info.insert([pos.x, pos.y], depth.0);
            });
    }

    let buf = match ron::to_string(&tile_info) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Could not serialize tile depth map [{}]", &e);
            return;
        }
    };

    let mut file = match File::options()
        .create(true)
        .write(true)
        .open("assets/depth_maps/tile_depth_map.ron")
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Could not open tile depth map file [{}]", &e);
            return;
        }
    };

    file.set_len(0);

    if file
        .write_all(buf.as_bytes())
        .map_err(|err| log::error!("Could not write to tile depth map file [{}]", err))
        .is_err()
    {
        return;
    };
    log::info!("Saved tile depth map!")
}

fn call_save_event(kb: Res<ButtonInput<KeyCode>>, mut saves: EventWriter<SaveEvent>) {
    if kb.just_pressed(KeyCode::KeyI) {
        log::info!("Saving tile depth map...");
        saves.send(SaveEvent);
    }
}
