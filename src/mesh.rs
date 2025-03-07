use crate::ldtk::{self, SpawnMeshEvent, TileDepth};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use bevy_ecs_tilemap::prelude::*;
use image::ImageBuffer;
use short_flight::ldtk::TileSlope;
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
}

#[derive(Default)]
struct MeshInfo {
    translation: Vec3,
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    texture_uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    texture_index: Option<usize>,
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
        }
    }
}

impl Plugin for TileMeshManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
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
    tiles: Query<(&TilePos, Option<&TileTextureIndex>, &TileDepth, &TileSlope)>,
) {
    for event in events.read() {
        log::info!("Spawning new mesh [{:?}]", event);

        let (tilemap_storage, tilemap_texture) = tilemaps.get(event.tilemap).unwrap();

        let image_handles = tilemap_texture.image_handles();

        let mut mesh_information = HashMap::new();
        for entity in tilemap_storage.iter().filter_map(|item| *item) {
            let Some(mesh_info) = create_mesh_from_tile_data(entity, &tiles) else {
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
        let hovering: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::GREEN));
        let selected: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::LIME));
        let missing: Handle<StandardMaterial> =
            asset_server.add(StandardMaterial::from_color(palettes::basic::PURPLE));

        let tilemap_mesh_data = TilemapMeshData {
            texture: (*texture).clone(),
            textures,
            tile_size: [tile_width, tile_height],
            texture_materials,
            hovering,
            selected,
            missing,
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
                .observe(set_tile_slope);

            mesh_bundle_inserts.insert(entity, bundle);
        }

        commands.insert_batch(mesh_bundle_inserts);
        commands.insert_resource(tilemap_mesh_data);
    }
}

fn update_individual_tile_mesh(
    mut commands: Commands,
    mut changed_tiles: EventReader<TileChanged>,
    tile_data_query: Query<(&TilePos, Option<&TileTextureIndex>, &TileDepth, &TileSlope)>,
    asset_server: Res<AssetServer>,
    tilemap_mesh_data: Option<Res<TilemapMeshData>>,
) {
    let Some(tilemap_mesh_data) = tilemap_mesh_data else {
        return;
    };
    for event in changed_tiles.read() {
        let Some(mesh_info) = create_mesh_from_tile_data(event.tile, &tile_data_query) else {
            log::error!("Could not create updated mesh from current tile data! (Wrong entity being queried?)");
            continue;
        };

        commands
            .entity(event.tile)
            .insert(get_mesh_components_from_info(
                &asset_server,
                &tilemap_mesh_data,
                mesh_info,
            ));
    }
}

fn get_mesh_components_from_info(
    asset_server: &Res<AssetServer>,
    tilemap_mesh_data: &TilemapMeshData,
    mesh_info: MeshInfo,
) -> (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform) {
    let MeshInfo {
        translation,
        vertices,
        indices,
        texture_uvs,
        normals,
        texture_index,
    } = mesh_info;

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices));

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
    );
    bundle
}

fn create_mesh_from_tile_data(
    tile: Entity,
    tile_data_query: &Query<(&TilePos, Option<&TileTextureIndex>, &TileDepth, &TileSlope)>,
) -> Option<MeshInfo> {
    let mut mesh: MeshInfo = Default::default();
    let MeshInfo {
        translation,
        vertices,
        indices,
        texture_uvs,
        normals,
        texture_index,
    } = &mut mesh;

    let (tile_pos, tile_texture, TileDepth(tile_depth), TileSlope(tile_slope)) =
        tile_data_query.get(tile).ok()?;

    *translation = Vec3::new(tile_pos.x as f32, 0.0, tile_pos.y as f32);

    let (vertex_data, index_data) = calculate_mesh_data(
        *tile_depth as f32,
        Vec3::from((*tile_slope, 0.)),
        0.0,
        [2; 4],
    );
    *vertices = vertex_data.clone().into_iter().map(|(v, _, _)| v).collect();
    *texture_uvs = vertex_data
        .clone()
        .into_iter()
        .map(|(_, uv, _)| uv)
        .collect();
    *normals = vertex_data.clone().into_iter().map(|(_, _, n)| n).collect();
    *indices = index_data;

    tile_texture.map(|texture| texture_index.insert(texture.0 as usize));

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
    mut transforms: Query<(&mut Transform)>,
    mut picking: ResMut<TilePickedMode>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if kb.pressed(KeyCode::ShiftLeft) {
        return;
    }
    picking.set(TilePickedMode::Move { tile: drag.target });
    if let TilePickedMode::Move { tile } = *picking {
        let mut transform = transforms.get_mut(drag.entity()).unwrap();
        transform.translation.y -= drag.delta.y * 0.01;
    }
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
    if kb.pressed(KeyCode::ShiftLeft) {
        return;
    }
    picking.set(TilePickedMode::Idle);
    let (mut depth, transform) = transforms.get_mut(drag.entity()).unwrap();
    **depth = transform.translation.y.ceil() as i64;
    tile_change_writer.send(TileChanged { tile: drag.target });
}

fn set_tile_slope(
    drag: Trigger<Pointer<bevy_picking::events::Drag>>,
    mut slopes: Query<&mut TileSlope>,
    mut tile_change_writer: EventWriter<TileChanged>,
    kb: Res<ButtonInput<KeyCode>>,
    mut picking: ResMut<TilePickedMode>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if !kb.pressed(KeyCode::ShiftLeft) {
        return;
    }
    let TilePickedMode::Drag { tile } = *picking else {
        return;
    };
    let mut slope = slopes.get_mut(drag.entity()).unwrap();
    if kb.just_pressed(KeyCode::KeyA) {
        slope.0.x -= 1.0;
        tile_change_writer.send(TileChanged { tile });
    }
    if kb.just_pressed(KeyCode::KeyD) {
        slope.0.x += 1.0;
        tile_change_writer.send(TileChanged { tile });
    }
    if kb.just_pressed(KeyCode::KeyW) {
        slope.0.y += 1.0;
        tile_change_writer.send(TileChanged { tile });
    }
    if kb.just_pressed(KeyCode::KeyS) {
        slope.0.y -= 1.0;
        tile_change_writer.send(TileChanged { tile });
    }
}

fn save_tile_data(
    _save: Trigger<GoSaveTheMesh>,
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

fn call_save_event(kb: Res<ButtonInput<KeyCode>>, mut saves: EventWriter<GoSaveTheMesh>) {
    if kb.just_pressed(KeyCode::KeyI) {
        log::info!("Saving tile depth map...");
        saves.send(GoSaveTheMesh);
    }
}

fn calculate_mesh_data(
    depth: f32,
    slope: Vec3,
    slope_i: f32,
    wall_counts: [u32; 4],
) -> (Vec<([f32; 3], [f32; 2], [f32; 3])>, Vec<u32>) {
    let x_pos: f32 = 0.0;
    let y_pos: f32 = 0.0;
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
    let mut insert = |data: (Vec<([f32; 3], [f32; 2], [f32; 3])>, Vec<u32>)| {
        vertices.append(&mut { data.0 });
        indices.append(&mut offset_indices(data.1));
    };

    let corners = get_slope_corner_depths(slope.xy(), slope_i);

    let top_vertices = top_vertices(x_pos, y_pos, corners);

    for (side_index, side) in
        // [[0, 1], [1, 2], [2, 3], [3, 0]]
        [[1, 0], [0, 3], [3, 2], [2, 1]].into_iter().enumerate()
    {
        let [v1, v2] = [top_vertices.0[side[0]].0, top_vertices.0[side[1]].0];
        insert(slope_wall_data(depth, v1, v2));

        for wall in 0..wall_counts[side_index] {
            let (new_vertices, new_indices) = wall_vertices(v1, v2, side_index);
            insert((
                new_vertices
                    .into_iter()
                    .map(|mut v| {
                        v.0[1] -= wall as f32;
                        v
                    })
                    .collect::<Vec<([f32; 3], [f32; 2], [f32; 3])>>(),
                new_indices,
            ));
        }
    }
    insert(top_vertices);

    vertices.iter_mut().for_each(|v| v.0[1] += depth);

    (vertices, indices)
}

/// generates the vertex data for a sloped wall, given:
/// - the base depth of the tile
/// - the two vertices that form the *top* line of the sloped wall
fn slope_wall_data(
    depth: f32,
    v1: [f32; 3],
    v2: [f32; 3],
) -> (Vec<([f32; 3], [f32; 2], [f32; 3])>, Vec<u32>) {
    let mut vertices: Vec<([f32; 3], [f32; 2], [f32; 3])> = vec![];
    let mut indices: Vec<u32> = vec![];

    let v1_is_slope = v1[1] > depth;
    let v2_is_slope = v2[1] > depth;
    if v1_is_slope {
        vertices.push({
            let mut v = v1.clone();
            v[1] = depth;
            (v, [1., 1.], [1., 1., 1.])
        });
    }
    if v2_is_slope {
        vertices.push({
            let mut v = v2.clone();
            v[1] = depth;
            (v, [0., 1.], [1., 1., 1.])
        });
    }
    if v1_is_slope | v2_is_slope {
        let max = f32::max(v1[1], v2[1]);
        vertices.push((v2.clone(), [0., (v1[1] - depth) / max], [1., 1., 1.]));
        vertices.push((v2.clone(), [1., (v2[1] - depth) / max], [1., 1., 1.]));
        if v1_is_slope & v2_is_slope {
            indices.append(&mut (vec![0, 1, 2, 0, 2, 3]));
        } else {
            indices.append(&mut (vec![0, 1, 2]));
        }
    }
    (vertices, indices)
}

/// generates the vertex data for the top of the mesh, given:
/// - the x and y position of the tile
/// - the height of each corner of the top face
fn top_vertices(
    x_pos: f32,
    y_pos: f32,
    corners: [f32; 4],
) -> (Vec<([f32; 3], [f32; 2], [f32; 3])>, Vec<u32>) {
    (
        vec![
            ([x_pos + 1., corners[1], y_pos], [1., 0.], [0., 1., 0.]), // tr
            ([x_pos, corners[0], y_pos], [0., 0.], [0., 1., 0.]),      // tl
            ([x_pos, corners[3], y_pos + 1.], [0., 1.], [0., 1., 0.]), // bl
            ([x_pos + 1., corners[2], y_pos + 1.], [1., 1.], [0., 1., 0.]), // br
        ],
        vec![0, 1, 2, 2, 3, 0],
    )
}

/// generates the vertex data for a wall, given:
/// - two vertices that form the *top* line for the wall
/// - a direction value to get the normals with
fn wall_vertices(
    wall_vertex_1: [f32; 3],
    wall_vertex_2: [f32; 3],
    side_index: usize,
) -> (Vec<([f32; 3], [f32; 2], [f32; 3])>, Vec<u32>) {
    let normal = match side_index {
        0 => [0., 0., -1.],
        1 => [1., 0., 0.],
        2 => [0., 0., 1.],
        3 => [-1., 0., 0.],
        _ => panic!(),
    };
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
        (wall_vertex_1.clone(), uv[0], normal.clone()),
        (wall_vertex_2.clone(), uv[1], normal.clone()),
        {
            let mut v = wall_vertex_2.clone();
            v[1] -= 1.0;
            (v, uv[2], normal.clone())
        },
        {
            let mut v = wall_vertex_1.clone();
            v[1] -= 1.0;
            (v, uv[3], normal.clone())
        },
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    (vertices, indices)
}

/// For the given slope (`s`) value, returns the depth of the four corners of a tile
/// that should become a slope.
///
/// The `i` parameter determines how "inclusive" a slope should be,
/// notably determining whether corner slopes become extrusive or intrusive.
///
/// This was written with `i` in mind being only within a range of 0.0-1.0,
/// and usually at only either end of the range,
/// but does not assert as much incase a unique `i` value proves to be useful.
fn get_slope_corner_depths(s: Vec2, i: f32) -> [f32; 4] {
    let c = |value: f32| f32::clamp(value, 0., f32::INFINITY);
    let cn = |value: f32| f32::clamp(value, f32::NEG_INFINITY, 0.);
    let tr = c(s.x) + cn(s.x) * i + c(s.y) + cn(s.y) * i;
    let bl = cn(s.x) + c(s.x) * i + cn(s.y) + c(s.y) * i;
    let br = c(s.x) + cn(s.x) * i + cn(s.y) + c(s.y) * i;
    let tl = cn(s.x) + c(s.x) * i + c(s.y) + cn(s.y) * i;
    [tl, tr, br, bl]
}
