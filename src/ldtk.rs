use crate::assets::ShortFlightLoadingState;
use crate::collision::{BasicCollider, ColliderShape, CollisionLayers, StaticCollision, ZHitbox};
use crate::npc;
use crate::npc::NPC;
use crate::tile::{TileDepth, TileFlags, TileSlope};
use bevy::color::palettes;
use bevy::ecs::system::SystemState;
use bevy::picking::pointer::PointerInteraction;
use bevy::platform::collections::HashMap;
use bevy::prelude::Asset;
use bevy::{asset::io::Reader, reflect::TypePath};
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext},
    prelude::*,
};
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_ecs_tilemap::map::{TilemapGridSize, TilemapType};
use bevy_ecs_tilemap::{
    TilemapBundle,
    map::{TilemapId, TilemapSize, TilemapTexture, TilemapTileSize},
    tiles::{TilePos, TileStorage, TileTextureIndex},
};
use serde::{Deserialize, Serialize};
use short_flight::deserialize_file;
use std::io::ErrorKind;
use std::path::PathBuf;
use thiserror::Error;

mod query;
mod tilemap_manager;

pub use query::TileQuery;
pub use tilemap_manager::TilemapManager;

const TILEMAP_PATH: &str = "assets/map_data/";

#[derive(AssetCollection, Resource)]
pub struct MapAssets {
    #[asset(path = "tilemap.ldtk")]
    pub map: Handle<LdtkMap>,
}

#[derive(Debug, Component, Reflect, Clone, Deref, DerefMut, PartialEq)]
pub struct LevelMetadataPath(String);

impl From<PathBuf> for LevelMetadataPath {
    fn from(value: PathBuf) -> Self {
        Self(value.to_string_lossy().to_string())
    }
}

impl LevelMetadataPath {
    pub fn with_extension(&self, extension: &str) -> PathBuf {
        let mut path_buf = PathBuf::from(&self.0);
        path_buf.add_extension(extension);
        path_buf
    }
}

#[derive(Default)]
pub struct LdtkPlugin;

impl Plugin for LdtkPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LdtkMap>()
            .add_event::<SpawnMeshEvent>()
            .register_asset_loader(LdtkLoader)
            .add_systems(
                Update,
                (
                    process_loaded_tile_maps.run_if(on_event::<AssetEvent<LdtkMap>>),
                    draw_mesh_intersections,
                    query::draw_chunk_labels
                        .run_if(|kb: Res<ButtonInput<KeyCode>>| kb.pressed(KeyCode::KeyC)),
                ),
            )
            .add_systems(OnEnter(ShortFlightLoadingState::Done), deferred_mesh_spawn);
    }
}

#[derive(TypePath, Asset)]
pub struct LdtkMap {
    pub project: ldtk_rust::Project,
    pub tilesets: HashMap<i64, Handle<Image>>,
}

#[derive(Default, Component, Clone)]
pub struct LdtkMapConfig {
    pub selected_level: usize,
}

#[derive(Default, Component, Clone)]
pub struct LdtkMapHandle(pub Handle<LdtkMap>);

#[derive(Default, Bundle)]
pub struct LdtkMapBundle {
    pub ldtk_map: LdtkMapHandle,
    pub ldtk_map_config: LdtkMapConfig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

fn deferred_mesh_spawn(mut commands: Commands, map: Res<MapAssets>) {
    log::info!("Spawned map");
    let ldtk_map = LdtkMapHandle(map.map.clone());
    let ldtk_map_config = LdtkMapConfig { selected_level: 1 };
    commands.spawn((
        LdtkMapBundle {
            ldtk_map: ldtk_map.clone(),
            ldtk_map_config: ldtk_map_config.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            global_transform: GlobalTransform::default(),
        },
        Name::new("LdtkMap"),
    ));
    commands.queue(move |world: &mut World| {
        let mut system_state = SystemState::<(
            Commands,
            EventReader<AssetEvent<LdtkMap>>,
            Res<Assets<LdtkMap>>,
            Query<(Entity, &LdtkMapHandle, &LdtkMapConfig)>,
        )>::new(world);

        let (mut commands, map_events, maps, query): (
            Commands<'_, '_>,
            EventReader<'_, '_, AssetEvent<LdtkMap>>,
            Res<'_, Assets<LdtkMap>>,
            Query<'_, '_, (Entity, &LdtkMapHandle, &LdtkMapConfig)>,
        ) = system_state.get(world);

        spawn_map(&mut commands, &maps, &ldtk_map, &ldtk_map_config);

        system_state.apply(world);
    });
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, palettes::tailwind::RED_500);
        gizmos.arrow(
            point,
            point + normal.normalize() * 0.5,
            palettes::tailwind::PINK_100,
        );
    }
}

#[derive(Debug, Event, Reflect, Clone)]
pub struct SpawnMeshEvent {
    pub tilemap: Entity,
}

pub struct LdtkLoader;

#[derive(Debug, Error)]
pub enum LdtkAssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load LDTk file: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for LdtkLoader {
    type Asset = LdtkMap;
    type Settings = ();
    type Error = LdtkAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        log::info!("Loading LdktMap asset via AssetLoader");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let project: ldtk_rust::Project = serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(
                ErrorKind::Other,
                format!("Could not read contents of Ldtk map: {e}"),
            )
        })?;
        let dependencies: Vec<(i64, AssetPath)> = project
            .defs
            .tilesets
            .iter()
            .filter_map(|tileset| {
                tileset.rel_path.as_ref().map(|rel_path| {
                    (
                        tileset.uid,
                        load_context.path().parent().unwrap().join(rel_path).into(),
                    )
                })
            })
            .collect();

        let ldtk_map = LdtkMap {
            project,
            tilesets: dependencies
                .iter()
                .map(|dep| (dep.0, load_context.load(dep.1.clone())))
                .collect(),
        };
        log::info!("Finished loading asset");
        Ok(ldtk_map)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["ldtk"];
        EXTENSIONS
    }
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    mut map_events: EventReader<AssetEvent<LdtkMap>>,
    maps: Res<Assets<LdtkMap>>,
    query: Query<(Entity, &LdtkMapHandle, &LdtkMapConfig)>,
    // new_maps: Query<&LdtkMapHandle, Added<LdtkMapHandle>>,
) {
    log::info!("Begun processing loaded tile maps");
    let mut event_maps = Vec::<AssetId<LdtkMap>>::default();
    for event in map_events.read() {
        match event {
            AssetEvent::Added { id } => {
                log::info!("Map added! {}", id);
                event_maps.push(*id);
            }
            AssetEvent::Modified { id } => {
                log::info!("Map changed! {}", id);
                event_maps.push(*id);
            }
            AssetEvent::Removed { id } => {
                log::info!("Map removed! {}", id);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                event_maps.retain(|changed_handle| changed_handle == id);
            }
            _ => continue,
        }
    }
    // log::info!("Events iterated");

    let changed_maps = event_maps.into_iter().filter_map(|changed_map| {
        query
            .iter()
            // only deal with currently changed map
            .find(|map_query| map_query.1.0.id() == changed_map)
            .map(|bundle| (changed_map, bundle))
    });

    // log::info!("{} Events iterated", changed_maps.clone().count());

    for (changed_map, (entity, map_handle, map_config)) in changed_maps {
        // Despawn all existing tilemaps for this LdtkMap
        commands.entity(entity).despawn_related::<Children>();

        spawn_map(&mut commands, &maps, map_handle, map_config);
    }
}

fn spawn_map(
    commands: &mut Commands,
    maps: &Res<Assets<LdtkMap>>,
    map_handle: &LdtkMapHandle,
    map_config: &LdtkMapConfig,
) {
    log::info!("Processing changed map!");

    let Some(ldtk_map) = maps.get(&map_handle.0) else {
        log::error!("Could not retrieve asset {:?}", map_handle.0);
        return;
    };

    let mut tilemap_manager = tilemap_manager::TilemapManager::from_project(&ldtk_map.project);

    spawn_map_components(commands, ldtk_map, map_config, &mut tilemap_manager);

    commands.insert_resource(tilemap_manager);
}

fn spawn_map_components(
    commands: &mut Commands,
    ldtk_map: &LdtkMap,
    map_config: &LdtkMapConfig,
    tilemap_manager: &mut tilemap_manager::TilemapManager,
) {
    // Pull out tilesets and their definitions into a new hashmap
    let mut tilesets = HashMap::new();
    ldtk_map.project.defs.tilesets.iter().for_each(|tileset| {
        let Some(get) = ldtk_map.tilesets.get(&tileset.uid) else {
            log::error!("Could not get tileset with uid of {}", tileset.uid);
            return;
        };
        log::info!("Tileset added: {}", tileset.uid);
        tilesets.insert(tileset.uid, (get.clone(), tileset));
    });

    let default_grid_size = ldtk_map.project.default_grid_size;
    let level = &ldtk_map.project.levels[map_config.selected_level];

    let map_tile_count_x = (level.px_wid / default_grid_size) as u32;
    let map_tile_count_y = (level.px_hei / default_grid_size) as u32;

    let size = TilemapSize {
        x: map_tile_count_x,
        y: map_tile_count_y,
    };

    let level_layer_instances = ldtk_map
        .project
        .levels
        .iter()
        .filter_map(|level| level.layer_instances.as_ref());

    // We will create a tilemap for each layer in the following loop
    for (layer_id, layer) in level_layer_instances.flatten().rev().enumerate() {
        let level = ldtk_map
            .project
            .levels
            .iter()
            .find(|level| level.uid == layer.level_id)
            .unwrap();

        fn get<T: for<'a> Deserialize<'a> + Default>(
            root_path: &PathBuf,
            extension: &str,
        ) -> HashMap<[u32; 2], T> {
            let mut r = root_path.clone();
            r.add_extension(extension);

            match deserialize_file::<HashMap<[u32; 2], T>>(&r) {
                Ok(some) => {
                    log::info!("Found {:?} data of {} tiles", r, some.len());
                    some
                }
                Err(_err) => {
                    log::error!("Could not find tile data for {:?}", r);
                    Default::default()
                }
            }
        }

        let root = PathBuf::from(TILEMAP_PATH.to_owned() + &level.identifier);

        let mut tile_depth_map = get::<TileDepth>(&root, "depth.ron");
        let mut tile_slope_map = get::<TileSlope>(&root, "slope.ron");
        let mut tile_flags_map = get::<TileFlags>(&root, "flags.ron");

        let metadata_path = LevelMetadataPath::from(root);

        let tilemap_position = Vec2::new(
            level.world_x as f32 / size.x as f32,
            level.world_y as f32 / size.y as f32,
        );

        let tilemap_transform = Transform::from_translation(tilemap_position.xxy().with_y(0.0));

        // Instantiate layer entities here
        for entity in layer.entity_instances.iter() {
            if entity.tags.contains(&"NPC".to_string()) {
                let id = entity
                    .field_instances
                    .iter()
                    .find(|field| field.identifier == "NPC_ID")
                    .expect("Expected to find NPC_ID field")
                    .value
                    .as_ref()
                    .expect("No value found for NPC_ID")
                    .as_u64()
                    .expect("Expected unsigned integer for NPC_ID, found something else");

                let name = entity
                    .field_instances
                    .iter()
                    .find(|field| field.identifier == "Name")
                    .map(|field| {
                        field
                            .value
                            .as_ref()
                            .unwrap_or(&serde_json::Value::String("[VOIDED]".to_string()))
                            .as_str()
                            .expect("Expected string value for Name, found something else")
                            .to_string()
                    });

                let depth = entity
                    .field_instances
                    .iter()
                    .find(|field| field.identifier == "Depth")
                    .map(|field| {
                        field
                            .value
                            .as_ref()
                            .unwrap_or(&serde_json::Value::Number(
                                serde_json::Number::from_f64(0.0).unwrap(),
                            ))
                            .as_f64()
                            .expect("Expected float value for Depth, found something else")
                    })
                    .unwrap_or_default();

                commands.queue(npc::commands::SpawnNPC {
                    npc_id: NPC::try_from(id as usize).unwrap(),
                    position: Vec3::new(
                        entity.px[0] as f32 / 32.,
                        depth as f32,
                        entity.px[1] as f32 / 32.,
                    ) + tilemap_transform.translation,
                    name,
                });
            }
        }

        let Some(uid) = layer.tileset_def_uid else {
            log::info!("Tileset uid not found for layer {}", layer_id);
            continue;
        };

        let (texture, tileset) = tilesets.get(&uid).unwrap().clone();

        // Tileset-specific tilemap settings
        let tile_size = TilemapTileSize {
            x: tileset.tile_grid_size as f32,
            y: tileset.tile_grid_size as f32,
        };

        // Pre-emptively create a map entity for tile creation
        let map_entity = commands.spawn_empty().id();

        // Create tiles for this layer from LDtk's grid_tiles and auto_layer_tiles
        let mut storage = TileStorage::empty(size);
        let mut children = vec![];

        // iterate over potential entities in this layer via layer.entity_instances

        for (index, tile) in layer
            .grid_tiles
            .iter()
            .chain(layer.auto_layer_tiles.iter())
            .enumerate()
        {
            let position = TilePos {
                x: ((tile.px[0]) / default_grid_size) as u32,
                y: ((tile.px[1]) / default_grid_size) as u32,
            };

            let key = [position.x, position.y];

            let tile_depth = tile_depth_map.remove(&key).unwrap_or_default();

            let tile_slope = tile_slope_map.remove(&key).unwrap_or_default();

            let tile_flags = tile_flags_map.remove(&key).unwrap_or(
                TileFlags::default() | TileFlags::from_bits(tile.f as u32).unwrap_or_default(),
            );

            let bundle = (
                Name::new(format!("Tile {}-{:?}", layer_id, position)),
                position,
                TilemapId(map_entity),
                TileTextureIndex(tile.t as u32),
                // BasicCollider::new(
                //     false,
                //     ColliderShape::Rect(Rect {
                //         min: Vec2 { x: 0., y: 0. },
                //         max: Vec2 { x: 1., y: 1. },
                //     }),
                //     CollisionLayers::Wall,
                //     CollisionLayers::all(),
                // ),
                // ZHitbox {
                //     y_tolerance: tile_slope.0.abs().max_element(),
                //     neg_y_tolerance: f32::NEG_INFINITY,
                // },
                // StaticCollision {},
                // Transform::from_translation(Vec3::ZERO),
                Transform::from_xyz(position.x as f32, tile_depth.f32(), position.y as f32),
                tile_depth,
                tile_slope,
                tile_flags,
                Visibility::Visible,
            );

            let tile_entity = commands.spawn(bundle).id();
            children.push(tile_entity.clone());
            storage.set(&position, tile_entity);
        }

        let tilemap = commands
            .entity(map_entity)
            .add_related::<ChildOf>(&children)
            .insert((
                // TilemapBundle {
                size,
                storage,
                tile_size,
                // grid_size:
                TilemapGridSize::from(tile_size),
                // map_type:
                TilemapType::default(),
                // texture:
                TilemapTexture::Single(texture),
                // transform:
                tilemap_transform,
                // anchor:
                bevy_ecs_tilemap::anchor::TilemapAnchor::default(),
                // ..default()
                // },
                metadata_path,
                Name::new(format!("Tilemap #{}", layer_id)),
            ))
            .id();

        tilemap_manager.insert(level, tilemap, commands);

        commands.send_event(SpawnMeshEvent { tilemap });
    }
}
