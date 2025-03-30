use crate::ldtk::{self, SpawnMeshEvent, TileDepth};
use crate::ldtk::{TileFlags, TileSlope};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::primitives::Aabb;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::SquareDirection;
use bevy_ecs_tilemap::prelude::*;
use image::ImageBuffer;
use short_flight::collision::ZHitbox;
use short_flight::serialize_to_file;
use std::collections::HashMap;

pub struct TileMeshManagerPlugin;

#[derive(Debug, Resource)]
struct TilemapMeshData {
    texture: Handle<Image>,
    textures: Vec<Handle<Image>>,
    tile_size: [u32; 2],
    texture_materials: Vec<Handle<StandardMaterial>>,
    hovering: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    missing: Handle<StandardMaterial>,
    mesh_cache: HashMap<MeshCacheKey, Handle<Mesh>>,
}

/// The state of a mesh that can be used to infer if an existing mesh can be reused.
///
/// List of properties efficient to cache with:
/// - Depth
/// - Slope
/// - Bit flags
/// - Texture
///
/// List of properties too inefficient to cache effectively. Meshes with any of these properties not set to their default values will not be cached.
/// - Walls
/// -
#[derive(Debug, Hash)]
struct MeshCacheKey {}

#[derive(Default)]
struct MeshInfo {
    vertices: Vec<[f32; 3]>,
    texture_uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    texture_index: Option<usize>,
    translation: Vec3,
}

#[derive(Debug, Clone, Copy, Event, Reflect)]
struct GoSaveTheMesh;

#[derive(Debug, Clone, Copy, Event, Reflect)]
struct TileChanged {
    tile: Entity,
}

#[derive(Debug, Resource, Reflect, Default)]
enum TilePickedMode {
    #[default]
    Idle,
    Move {
        tile: Entity,
    },
    Drag {
        tile: Entity,
    },
    Paint {
        selected: Entity,
        painting: Option<Entity>,
    },
}

impl TilePickedMode {
    fn set(&mut self, mode: TilePickedMode) {
        match self {
            TilePickedMode::Idle => *self = mode,
            TilePickedMode::Move { tile: _ } => {
                if let TilePickedMode::Idle = mode {
                    *self = TilePickedMode::Idle
                }
            }
            TilePickedMode::Drag { tile: _ } => {
                if let TilePickedMode::Idle = mode {
                    *self = TilePickedMode::Idle
                }
            }
            TilePickedMode::Paint { selected, painting } => {}
        }
    }
}

impl Plugin for TileMeshManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                adjust_tiles_via_keystrokes,
                update_individual_tile_mesh,
                spawn_massive_tilemap_mesh.after(ldtk::process_loaded_tile_maps),
                (
                    call_save_event,
                    |mut commands: Commands, mut event_reader: EventReader<GoSaveTheMesh>| {
                        for event in event_reader.read() {
                            commands.trigger(*event);
                        }
                    },
                )
                    .chain(),
                // manage_tile_refresh_events,
            ),
        )
        .add_event::<GoSaveTheMesh>()
        .add_event::<TileChanged>()
        .insert_resource(TilePickedMode::default())
        // .add_observer(|trigger: Trigger<>|)
        .add_observer(save_tile_data);
    }
}

fn spawn_massive_tilemap_mesh(
    mut events: EventReader<SpawnMeshEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    tilemaps: Query<(&TileStorage, &TilemapTexture)>,
    tiles: Query<(
        &TilePos,
        &TileTextureIndex,
        &TileDepth,
        &TileSlope,
        &TilemapId,
        &TileFlags,
    )>,
    tilemap_query: Query<(&TileStorage, &TilemapSize)>,
) {
    for event in events.read() {
        log::info!("Spawning new mesh [{:?}]", event);

        let (tilemap_storage, tilemap_texture) = tilemaps.get(event.tilemap).unwrap();

        let image_handles = tilemap_texture.image_handles();

        let mut mesh_information = HashMap::new();
        for entity in tilemap_storage.iter().filter_map(|item| *item) {
            let Some(mesh_info) = create_mesh_from_tile_data(entity, &tiles, &tilemap_query) else {
                log::error!("Could not query tile info for {}'s mesh data!", entity);
                continue;
            };
            mesh_information.insert(entity, mesh_info);
        }

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
            .iter()
            .map(|image_handle| asset_server.add(StandardMaterial::from(image_handle.clone())))
            .collect();
        let hovering: Handle<StandardMaterial> = asset_server.add({
            let mut material = StandardMaterial::from_color(palettes::basic::GREEN);
            material.cull_mode = None;
            material
        });
        let selected: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::LIME));
        let missing: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::PURPLE));

        let mut tilemap_mesh_data = TilemapMeshData {
            texture: (*texture).clone(),
            textures,
            tile_size: [tile_width, tile_height],
            texture_materials,
            hovering,
            selected,
            missing,
            mesh_cache: HashMap::new(),
        };

        let tilemap_mesh_data_ref: &TilemapMeshData = &tilemap_mesh_data;

        let mut mesh_bundle_inserts = HashMap::new();

        for (entity, mesh_info) in mesh_information {
            let bundle =
                get_mesh_components_from_info(&asset_server, tilemap_mesh_data_ref, mesh_info);

            commands
                .entity(entity)
                .observe(update_material_on::<Pointer<Over>>(
                    tilemap_mesh_data_ref.hovering.clone(),
                ))
                .observe(update_material_on::<Pointer<Out>>(bundle.1 .0.clone()))
                .observe(update_material_on::<Pointer<Down>>(
                    tilemap_mesh_data_ref.selected.clone(),
                ))
                .observe(update_material_on::<Pointer<Up>>(
                    tilemap_mesh_data_ref.hovering.clone(),
                ))
                .observe(move_on_drag)
                .observe(adjust_on_release)
                .observe(adjust_on_up)
                .observe(select_tile_for_modifying)
                .observe(select_tile_for_painting);

            mesh_bundle_inserts.insert(entity, bundle);
        }

        commands.insert_batch(mesh_bundle_inserts);
        commands.insert_resource(tilemap_mesh_data);
    }
}

fn update_individual_tile_mesh(
    mut commands: Commands,
    mut changed_tiles: EventReader<TileChanged>,
    tile_data_query: Query<(
        &TilePos,
        &TileTextureIndex,
        &TileDepth,
        &TileSlope,
        &TilemapId,
        &TileFlags,
    )>,
    tilemap_query: Query<(&TileStorage, &TilemapSize)>,
    asset_server: Res<AssetServer>,
    tilemap_mesh_data: Option<Res<TilemapMeshData>>,
) {
    let Some(tilemap_mesh_data) = tilemap_mesh_data else {
        return;
    };
    for event in changed_tiles.read() {
        let key = {
            let key = MeshCacheKey {};

            key
        };

        let Some(mesh_info) =
            create_mesh_from_tile_data(event.tile, &tile_data_query, &tilemap_query)
        else {
            log::error!("Could not create updated mesh from current tile data! (Wrong entity being queried?)");
            continue;
        };

        let enclosing = Aabb::enclosing(
            mesh_info
                .vertices
                .iter()
                .map(|value| Vec3::from(*value))
                .collect::<Vec<Vec3>>(),
        )
        .unwrap();
        commands.entity(event.tile).insert((
            get_mesh_components_from_info(&asset_server, &tilemap_mesh_data, mesh_info),
            enclosing,
        ));
    }
}

fn get_mesh_components_from_info(
    asset_server: &Res<AssetServer>,
    tilemap_mesh_data: &TilemapMeshData,
    mesh_info: MeshInfo,
) -> (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, ZHitbox) {
    let MeshInfo {
        translation,
        vertices,
        indices,
        texture_uvs,
        texture_index,
    } = mesh_info;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
    .with_inserted_indices(Indices::U32(indices));

    mesh.duplicate_vertices();
    mesh.compute_flat_normals();

    let material = texture_index
        .map(|texture_index| {
            tilemap_mesh_data
                .texture_materials
                .get(texture_index)
                .unwrap()
                .clone()
        })
        .unwrap_or(tilemap_mesh_data.missing.clone());

    let bundle = (
        Mesh3d(asset_server.add(mesh)),
        MeshMaterial3d(material),
        Transform::from_translation(translation),
        ZHitbox {
            y_tolerance: 0.0,
            neg_y_tolerance: f32::NEG_INFINITY,
        },
    );
    bundle
}

fn create_mesh_from_tile_data(
    tile: Entity,
    tile_data_query: &Query<(
        &TilePos,
        &TileTextureIndex,
        &TileDepth,
        &TileSlope,
        &TilemapId,
        &TileFlags,
    )>,
    tilemap_query: &Query<(&TileStorage, &TilemapSize)>,
) -> Option<MeshInfo> {
    let mut mesh: MeshInfo = Default::default();
    let MeshInfo {
        translation,
        vertices,
        indices,
        texture_uvs,
        texture_index,
    } = &mut mesh;

    let (tile_pos, tile_texture, tile_depth, TileSlope(tile_slope), tilemap_id, tile_flags) =
        tile_data_query.get(tile).ok()?;

    *translation = Vec3::new(tile_pos.x as f32, tile_depth.f32(), tile_pos.y as f32);

    let (tile_storage, map_size) = tilemap_query
        .get(tilemap_id.0)
        .expect("Expected tilemap id to be valid");
    let wall_counts: [u32; 4] = [
        SquareDirection::South,
        SquareDirection::East,
        SquareDirection::North,
        SquareDirection::West,
    ]
    .map(|direction| tile_pos.square_offset(&direction, map_size))
    .map(|neighbor_pos| {
        neighbor_pos
            .map(|value| tile_storage.get(&value))
            .unwrap_or(None)
    })
    .map(|neighbor_entity| {
        neighbor_entity
            .map(|entity| tile_data_query.get(entity).ok())
            .unwrap_or(None)
    })
    .map(|neighbor_components| {
        neighbor_components.map(|components| tile_depth.f32() - components.2.f32())
    })
    .map(|height_difference| {
        height_difference
            .map(|value| value.clamp(0., f32::INFINITY).ceil() as u32)
            .unwrap_or(200)
    });

    let (vertex_data, index_data) = calculate_mesh_data(*tile_slope, wall_counts, *tile_flags);

    (*vertices, *texture_uvs) = vertex_data.into_iter().unzip();
    *indices = index_data;

    *texture_index.insert(tile_texture.0 as usize);

    return Some(mesh);
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
    mut transforms: Query<(&mut Transform, &mut TileSlope)>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if kb.pressed(KeyCode::ShiftLeft) {
        let (_, mut slope) = transforms.get_mut(drag.entity()).unwrap();

        slope.0.y -= drag.delta.y * 0.01;

        return;
    }
    picking.set(TilePickedMode::Move { tile: drag.target });
    let (mut transform, _) = transforms.get_mut(drag.entity()).unwrap();

    transform.translation.y -= drag.delta.y * 0.01;
}

fn adjust_on_release(
    drag: Trigger<Pointer<DragEnd>>,
    mut transforms: Query<(&mut TileDepth, &Transform)>,
    mut tile_change_writer: EventWriter<TileChanged>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    picking.set(TilePickedMode::Idle);
    let (mut depth, transform) = transforms.get_mut(drag.entity()).unwrap();
    *depth = TileDepth::from(depth.f32() + transform.translation.y.round());
    tile_change_writer.send(TileChanged { tile: drag.target });
}

fn adjust_on_up(
    drag: Trigger<Pointer<Up>>,
    mut transforms: Query<(&mut TileDepth, &Transform)>,
    mut tile_change_writer: EventWriter<TileChanged>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    picking.set(TilePickedMode::Idle);
    let (mut depth, transform) = transforms.get_mut(drag.entity()).unwrap();
    *depth = TileDepth::from(depth.f32() + transform.translation.y.round());
    tile_change_writer.send(TileChanged { tile: drag.target });
}

fn select_tile_for_modifying(
    drag: Trigger<Pointer<Down>>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
    tile_data: Query<(&TilePos, &TileDepth, &TileSlope, &TileFlags)>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if !kb.pressed(KeyCode::ShiftLeft) {
        return;
    }

    let TilePickedMode::Idle = *picking else {
        return;
    };
    let tile = drag.target;

    *picking = TilePickedMode::Move { tile };

    let (pos, depth, slope, flags) = tile_data.get(tile).unwrap();

    log::info!(
        "Selected! target: [{:?}] depth: [{}] slope [{}] flags: [{}]",
        [pos.x, pos.y],
        depth.f32(),
        slope.0,
        flags
    )
}

fn select_tile_for_painting(
    drag: Trigger<Pointer<Over>>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
    tile_data: Query<(&TilePos, &TileDepth, &TileSlope, &TileFlags)>,
) {
    let TilePickedMode::Paint { selected, painting } = *picking else {
        return;
    };

    if drag.target == selected {
        return;
    }

    *picking = TilePickedMode::Paint {
        selected,
        painting: Some(drag.target),
    };
}

fn adjust_tiles_via_keystrokes(
    mut tile_data: Query<(&mut TileDepth, &mut TileSlope, &mut TileFlags, &Transform)>,
    mut tile_change_writer: EventWriter<TileChanged>,
    kb: Res<ButtonInput<KeyCode>>,
    mut picking: ResMut<TilePickedMode>,
    mut gizmos: Gizmos,
) {
    if let TilePickedMode::Paint { selected, painting } = &mut *picking {
        if kb.just_pressed(KeyCode::KeyE) {
            *picking = TilePickedMode::Idle;
            log::info!("Stopped painting");
        } else if painting.is_some() && kb.pressed(KeyCode::Space) {
            log::info!("Painted {} {:?}", selected, painting);
            let [s, mut p] = tile_data.many_mut([*selected, painting.unwrap()]);
            *p.0 = s.0.clone();
            *p.1 = s.1.clone();
            *p.2 = s.2.clone();

            tile_change_writer.send(TileChanged {
                tile: painting.unwrap(),
            });

            *painting = None;
        }
    };

    let TilePickedMode::Move { tile } = *picking else {
        return;
    };

    let (depth, mut slope, mut rotate_mode, transform) = tile_data.get_mut(tile).unwrap();

    gizmos.grid_3d(
        Vec3::new(
            transform.translation.x,
            transform.translation.y.round(),
            transform.translation.z,
        ),
        UVec3::new(40, 1, 40),
        Vec3::ONE,
        palettes::basic::RED,
    );

    if kb.just_pressed(KeyCode::KeyA) {
        slope.0.x -= 0.25;
        tile_change_writer.send(TileChanged { tile });
        log::info!("Changed slope -x");
    } else if kb.just_pressed(KeyCode::KeyD) {
        slope.0.x += 0.25;
        tile_change_writer.send(TileChanged { tile });
        log::info!("Changed slope +x");
    } else if kb.just_pressed(KeyCode::KeyW) {
        slope.0.z += 0.25;
        tile_change_writer.send(TileChanged { tile });
        log::info!("Changed slope +y");
    } else if kb.just_pressed(KeyCode::KeyS) {
        slope.0.z -= 0.25;
        tile_change_writer.send(TileChanged { tile });
        log::info!("Changed slope -y");
    } else if kb.just_pressed(KeyCode::KeyQ) {
        *picking = TilePickedMode::Paint {
            selected: tile,
            painting: None,
        };
        log::info!("Started painting with {}", tile);
    } else {
        let flags = (0..8)
            .into_iter()
            .map(|i| TileFlags::from_bits(1 << i).unwrap_or_default())
            .zip([
                KeyCode::KeyZ,
                KeyCode::KeyX,
                KeyCode::KeyC,
                KeyCode::KeyV,
                KeyCode::KeyB,
                KeyCode::KeyN,
                KeyCode::KeyM,
                KeyCode::Comma,
            ]);
        for (flag, key) in flags {
            if kb.just_pressed(key) {
                *rotate_mode ^= flag;
                let flag_val = if rotate_mode.contains(flag) { "+" } else { "-" };
                log::info!("Changed flag {}{}", flag_val, flag);
                tile_change_writer.send(TileChanged { tile });
            }
        }
    }
}

/// Saves the depth and slope data for every tile in a tilemap.
///
/// Depth and slope values that are equal to their defaults are ommitted for the sake of clarity,
/// since values not given are initialized to their defaults anyway.
fn save_tile_data(
    _save: Trigger<GoSaveTheMesh>,
    tilemaps: Query<&TileStorage>,
    tiles: Query<(&TilePos, &TileDepth, &TileSlope, &TileFlags)>,
) {
    let mut depth_info: HashMap<[u32; 2], &TileDepth> = HashMap::new();
    let mut slope_info: HashMap<[u32; 2], &TileSlope> = HashMap::new();
    let mut flag_info: HashMap<[u32; 2], &TileFlags> = HashMap::new();
    for tilemap in tilemaps.iter() {
        tilemap
            .iter()
            .filter_map(|item| item.map(|item| tiles.get(item).ok()).unwrap_or_default())
            .for_each(|(pos, depth, slope, flags)| {
                if depth.f32() != 0.0 {
                    depth_info.insert([pos.x, pos.y], depth);
                }
                if slope.0 != Vec3::ZERO {
                    slope_info.insert([pos.x, pos.y], slope);
                }
                if !flags.is_empty() {
                    flag_info.insert([pos.x, pos.y], flags);
                }
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
    let path = "assets/depth_maps/tile_flag_map.ron";
    if serialize_to_file(flag_info, path) {
        log::info!("Saved tile flag map!")
    }
}

fn call_save_event(kb: Res<ButtonInput<KeyCode>>, mut saves: EventWriter<GoSaveTheMesh>) {
    if kb.just_pressed(KeyCode::KeyI) {
        log::info!("Saving tile depth map...");
        saves.send(GoSaveTheMesh);
    }
}

fn calculate_mesh_data(
    slope: Vec3,
    wall_counts: [u32; 4],
    flags: TileFlags,
) -> (Vec<([f32; 3], [f32; 2])>, Vec<u32>) {
    let mut vertices = vec![];
    let mut indices = vec![];

    let mut index_offset = 0;
    let mut offset_indices = |indices: Vec<u32>| {
        let maximum = indices.iter().max().map(|i| *i + 1).unwrap_or_default();
        let new: Vec<u32> = indices
            .into_iter()
            .map(|value| value + index_offset)
            .collect();
        index_offset += maximum;
        new
    };
    let mut insert = |data: (Vec<([f32; 3], [f32; 2])>, Vec<u32>)| {
        vertices.append(&mut { data.0 });
        indices.append(&mut offset_indices(data.1));
    };

    let corners = get_slope_corner_depths(slope, !flags.intersects(TileFlags::Exclusive));

    let top_vertices = top_vertices(corners, flags);

    for (side_index, side) in [[1, 0], [0, 3], [3, 2], [2, 1]].into_iter().enumerate() {
        let [v1, v2] = [top_vertices.0[side[0]].0, top_vertices.0[side[1]].0];
        insert(slope_wall_data(v1, v2));

        for wall in 0..wall_counts[side_index] {
            let (new_vertices, new_indices) = wall_vertices(v1, v2, side_index);
            insert((
                new_vertices
                    .into_iter()
                    .map(|mut v| {
                        v.0[1] -= wall as f32;
                        v
                    })
                    .collect::<Vec<([f32; 3], [f32; 2])>>(),
                new_indices,
            ));
        }
    }
    insert(top_vertices);

    (vertices, indices)
}

/// For the given slope (`s`) value, returns the depth of the four corners of a tile
/// that should become a slope.
///
/// `inclusive` determines the shape a slope will take,
/// notably determining whether corner slopes become extrusive or intrusive.
pub(crate) fn get_slope_corner_depths(s: Vec3, inclusive: bool) -> [f32; 4] {
    let i = inclusive as i32 as f32;
    let pos: Box<dyn Fn(f32) -> f32> = Box::new(|value: f32| f32::clamp(value, 0., f32::INFINITY));
    let neg: Box<dyn Fn(f32) -> f32> =
        Box::new(|value: f32| -f32::clamp(value, f32::NEG_INFINITY, 0.));

    let points = [s.z; 4];
    let mapping = [
        (&neg, &pos), // tl
        (&pos, &pos), // tr
        (&pos, &neg), // br
        (&neg, &neg), // bl
    ];

    let select_corner =
        |(point, map): (f32, (&Box<dyn Fn(f32) -> f32>, &Box<dyn Fn(f32) -> f32>))| {
            let x_component = map.0(s.x);
            let z_component = map.1(s.z);

            let mut total = 0.;

            if x_component == 0. {
                total += z_component * i
            } else if z_component == 0. {
                total += x_component * i
            } else {
                total += (x_component + z_component) / 2.
            }

            point + total.clamp(0., f32::INFINITY)
        };

    let mut a = points.into_iter().zip(mapping).map(select_corner);

    [
        a.next().unwrap(), // tl
        a.next().unwrap(), // tr
        a.next().unwrap(), // br
        a.next().unwrap(), // bl
    ]
}

/// generates the vertex data for the top of the mesh, given:
/// - the x and y position of the tile
/// - the height of each corner of the top face
fn top_vertices(
    corners: [f32; 4],
    rotate_mode: TileFlags,
) -> (Vec<([f32; 3], [f32; 2])>, Vec<u32>) {
    let mut vertices = vec![
        ([1., corners[1], 0.]), // tr
        ([0., corners[0], 0.]), // tl
        ([0., corners[3], 1.]), // bl
        ([1., corners[2], 1.]), // br
    ];
    let mut uvs = vec![[1., 0.], [0., 0.], [0., 1.], [1., 1.]];
    let mut indices = match rotate_mode {
        flags if flags.intersects(TileFlags::FlipTriangles & TileFlags::Fold) => [0, 1, 2, 2, 3, 0],
        flags if flags.intersects(TileFlags::FlipTriangles) => [0, 1, 2, 2, 3, 0],
        flags if flags.intersects(TileFlags::Fold) => [0, 1, 2, 2, 3, 0],
        _ => [0, 1, 2, 2, 3, 0],
    }
    .to_vec();

    //(1,0)
    //(2,3)
    if rotate_mode.intersects(TileFlags::FlipX) {
        uvs.swap(1, 0);
        uvs.swap(2, 3);
    }
    if rotate_mode.intersects(TileFlags::FlipY) {
        uvs.swap(1, 2);
        uvs.swap(0, 3);
    }
    if rotate_mode.intersects(TileFlags::FlipTriangles) {
        // 012,230 -> 123,301
        indices = vec![1, 2, 3, 3, 0, 1];
    }
    if rotate_mode.intersects(TileFlags::Fold) {
        vertices.append(&mut vec![
            vertices[0].clone(), // tr2
            vertices[2].clone(), // bl2
        ]);
        uvs.append(&mut vec![
            uvs[0].clone(), // tr2
            uvs[2].clone(), // bl2
        ]);

        indices = vec![0, 1, 2, 5, 3, 4];
    }

    let vertices = (0..vertices.len()).map(|i| (vertices[i], uvs[i])).collect();

    (vertices, indices)
}

/// generates the vertex data for a sloped wall, given:
/// - the base depth of the tile
/// - the two vertices that form the *top* line of the sloped wall
fn slope_wall_data(v1: [f32; 3], v2: [f32; 3]) -> (Vec<([f32; 3], [f32; 2])>, Vec<u32>) {
    let depth = 0.0;
    let mut vertices: Vec<([f32; 3], [f32; 2])> = vec![];
    let mut indices: Vec<u32> = vec![];

    let v1_is_slope = v1[1] > depth;
    let v2_is_slope = v2[1] > depth;
    if v1_is_slope {
        vertices.push({
            let mut v = v1.clone();
            v[1] = depth;
            (v, [1., 1.])
        });
    }
    if v2_is_slope {
        vertices.push({
            let mut v = v2.clone();
            v[1] = depth;
            (v, [0., 1.])
        });
    }
    if v1_is_slope | v2_is_slope {
        let max = f32::max(v1[1], v2[1]);
        vertices.push((v2.clone(), [0., (v1[1] - depth) / max]));
        vertices.push((v2.clone(), [1., (v2[1] - depth) / max]));
        if v1_is_slope & v2_is_slope {
            indices.append(&mut (vec![0, 1, 2, 0, 2, 3]));
        } else {
            indices.append(&mut (vec![0, 1, 2]));
        }
    }
    (vertices, indices)
}

/// generates the vertex data for a wall, given:
/// - two vertices that form the *top* line for the wall
/// - a direction value to get the normals with
fn wall_vertices(
    wall_vertex_1: [f32; 3],
    wall_vertex_2: [f32; 3],
    side_index: usize,
) -> (Vec<([f32; 3], [f32; 2])>, Vec<u32>) {
    let uv = match side_index {
        // Back
        0 => [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        // Front
        2 => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        // Right
        1 => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        // Left
        3 => [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        // Top
        4 => [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        // Bottom
        5 => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        _ => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    };
    let vertices = vec![
        (wall_vertex_1.clone(), uv[0]),
        (wall_vertex_2.clone(), uv[1]),
        {
            let mut v = wall_vertex_2.clone();
            v[1] -= 1.0;
            (v, uv[2])
        },
        {
            let mut v = wall_vertex_1.clone();
            v[1] -= 1.0;
            (v, uv[3])
        },
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    (vertices, indices)
}
