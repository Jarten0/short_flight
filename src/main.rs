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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(TilemapPlugin)
        .add_plugins(LdtkPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(DefaultEditorCamPlugins)
        .add_systems(PreStartup, setup)
        .add_systems(Startup, spawn_protag)
        .add_systems(
            Update,
            (
                update_protag,
                spawn_mesh.after(process_loaded_tile_maps),
                draw_mesh_intersections,
            ),
        )
        .add_event::<SpawnMeshEvent>()
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        // DirectionalLight::default(),
        // Transform::default().looking_to(Vec3::NEG_Y, Dir3::Y),
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
            .with_translation(Vec3::new(0.0, 20.0, 0.0)),
        EditorCam::default().with_initial_anchor_depth(20.0),
    ));
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .observe(move_on_drag);

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

fn spawn_mesh(
    mut events: EventReader<SpawnMeshEvent>,
    mut commands: Commands,
    mut asset_server: Res<AssetServer>,
    maps: Res<Assets<LdtkMap>>,
    images: Res<Assets<Image>>,
    map_asset: Single<&LdtkMapHandle>,
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
            ),
        > = HashMap::new();

        let (tilemap_storage, tilemap_texture) = tilemaps.get(event.tilemap).unwrap();

        for entity in tilemap_storage.iter().filter_map(|item| *item) {
            let (tile_pos, tile_texture, tile_depth) = tiles.get(entity).unwrap();
            let x_pos = -(tile_pos.x as f32);
            let y_pos = tile_pos.y as f32;

            mesh_information.insert(entity, Default::default());

            // pull a reference instead of declaring so that changing type declaration is easier
            let (
                ref mut vertices,
                ref mut indices,
                ref mut texture_uvs,
                ref mut normals,
                ref mut texture_index,
            ) = mesh_information.get_mut(&entity).unwrap();

            let z_pos = 0.0;
            vertices.push([x_pos, z_pos, y_pos]);
            vertices.push([x_pos, z_pos, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, z_pos, y_pos + 1.0]);
            vertices.push([x_pos + 1.0, z_pos, y_pos]);
            texture_uvs.push([0.0, 0.0]);
            texture_uvs.push([0.0, 1.0]);
            texture_uvs.push([1.0, 1.0]);
            texture_uvs.push([1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            tile_texture.map(|texture| texture_index.insert(texture.0 as usize));

            const FACE_INDICES: &[u32] = &[0, 1, 2, 0, 2, 3];

            let offseted: &mut Vec<u32> = &mut FACE_INDICES
                .to_owned()
                .into_iter()
                .map(|value: u32| value + (vertices.len() as u32 - 4))
                .collect();
            indices.append(offseted);
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

        for (entity, (vertices, indices, texture_uvs, normals, texture_index)) in mesh_information {
            // log::info!("spawning mesh with {} vertices, {} indices, {} texture uvs, and this texture index {:?}", vertices.len(), indices.len(), texture_uvs.len(), texture_index);

            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_indices(Indices::U32(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

            let material = texture_index
                .map(|texture_index| texture_materials.get(texture_index).unwrap().clone())
                // .unwrap_or(None)
                .unwrap_or(missing.clone());

            commands
                .entity(entity)
                .observe(update_material_on::<Pointer<Over>>(hovering.clone()))
                .observe(update_material_on::<Pointer<Out>>(material.clone()))
                .observe(update_material_on::<Pointer<Down>>(selected.clone()))
                .observe(update_material_on::<Pointer<Up>>(hovering.clone()))
                .observe(move_on_drag);

            mesh_bundle_inserts.insert(
                entity,
                (
                    Mesh3d(asset_server.add(mesh)),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    // Name::new(format!("DrawMesh {:?}", texture_index)),
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
    // mut camera: Single<&mut EditorCam>,
) {
    // camera.enabled_motion.pan = false;
    let mut transform = transforms.get_mut(drag.entity()).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
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
