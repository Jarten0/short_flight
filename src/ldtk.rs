use bevy::color::palettes;
use bevy::prelude::Asset;
use bevy_ecs_tilemap::{
    helpers::geometry::get_tilemap_center_transform,
    map::{TilemapId, TilemapSize, TilemapTexture, TilemapTileSize},
    tiles::{TileBundle, TilePos, TileStorage, TileTextureIndex},
    TilemapBundle,
};
use std::ops::Deref;
use std::{collections::HashMap, io::ErrorKind};
use thiserror::Error;

use bevy::{asset::io::Reader, reflect::TypePath};
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext},
    prelude::*,
};
use bevy_ecs_tilemap::map::TilemapType;

use crate::deserialize_file;

#[derive(Default)]
pub struct LdtkPlugin;

impl Plugin for LdtkPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LdtkMap>()
            .register_asset_loader(LdtkLoader)
            .add_systems(Update, process_loaded_tile_maps);
    }
}

#[derive(TypePath, Asset)]
pub struct LdtkMap {
    pub project: ldtk_rust::Project,
    pub tilesets: HashMap<i64, Handle<Image>>,
}

#[derive(Default, Component)]
pub struct LdtkMapConfig {
    pub selected_level: usize,
}

#[derive(Default, Component)]
pub struct LdtkMapHandle(pub Handle<LdtkMap>);

#[derive(Default, Bundle)]
pub struct LdtkMapBundle {
    pub ldtk_map: LdtkMapHandle,
    pub ldtk_map_config: LdtkMapConfig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
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
    // log::info!("Begun processing loaded tile maps");
    let mut changed_maps = Vec::<AssetId<LdtkMap>>::default();
    for event in map_events.read() {
        match event {
            AssetEvent::Added { id } => {
                log::info!("Map added! {}", id);
                changed_maps.push(*id);
            }
            AssetEvent::Modified { id } => {
                log::info!("Map changed! {}", id);
                changed_maps.push(*id);
            }
            AssetEvent::Removed { id } => {
                log::info!("Map removed! {}", id);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.retain(|changed_handle| changed_handle == id);
            }
            _ => continue,
        }
    }
    // log::info!("Events iterated");

    // If we have new map entities, add them to the changed_maps list
    // let mut other: Vec<AssetId<LdtkMap>> = new_maps
    //     .iter()
    //     .map(|new_map_handle| new_map_handle.0.id())
    //     .collect();
    // changed_maps.append(&mut other);

    let changed_maps = changed_maps.iter().filter_map(|changed_map| {
        query
            .iter()
            // only deal with currently changed map
            .find(|a| a.1 .0.id() == *changed_map)
            .map(|bundle| (changed_map, bundle))
    });

    // log::info!("{} Events iterated", changed_maps.clone().count());

    for (changed_map, (entity, map_handle, map_config)) in changed_maps {
        log::info!("Processing changed map!");
        assert!(
            map_handle.0.id() == *changed_map,
            "Invalid deviation from the example"
        );

        let Some(ldtk_map) = maps.get(&map_handle.0) else {
            log::error!("Could not retrieve asset {:?}", map_handle.0);
            return;
        };

        // Despawn all existing tilemaps for this LdtkMap
        commands.entity(entity).despawn_descendants();

        spawn_map_components(&mut commands, ldtk_map, map_config);
    }
}

fn spawn_map_components(commands: &mut Commands, ldtk_map: &LdtkMap, map_config: &LdtkMapConfig) {
    let tile_depth_map: TileDepthMapSerialization =
        deserialize_file("assets/depth_maps/tile_depth_map.ron").unwrap_or_default();
    let tile_slope_map: TileSlopeMapSerialization =
        deserialize_file("assets/depth_maps/tile_slope_map.ron").unwrap_or_default();

    log::info!("Found tilemap depth data of {} tiles", tile_depth_map.len());
    log::info!("Found tilemap slope data of {} tiles", tile_slope_map.len());

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

    // We will create a tilemap for each layer in the following loop
    for (layer_id, layer) in level
        .layer_instances
        .as_ref()
        .unwrap()
        .iter()
        .rev()
        .enumerate()
    {
        let Some(uid) = layer.tileset_def_uid else {
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

            let tile_depth = tile_depth_map
                .get(&[position.x, position.y])
                .map(|item| *item)
                .unwrap_or_default();

            let tile_slope = tile_slope_map
                .get(&[position.x, position.y])
                .map(|item| *item)
                .unwrap_or_default();

            let bundle = (
                TileBundle {
                    position,
                    tilemap_id: TilemapId(map_entity),
                    texture_index: TileTextureIndex(tile.t as u32),
                    // Hidden since we use a mesh to draw them in 3d
                    // visible: TileVisible(false),
                    ..default()
                },
                TileDepth(tile_depth),
                TileSlope(Vec2::from(tile_slope)),
                Name::new(format!("Tile {}-{}", uid, index)),
            );

            storage.set(&position, commands.spawn(bundle).id());
        }

        let tilemap = commands
            .entity(map_entity)
            .insert((
                TilemapBundle {
                    grid_size: tile_size.into(),
                    map_type: TilemapType::default(),
                    size,
                    storage,
                    texture: TilemapTexture::Single(texture),
                    tile_size,
                    transform: Transform::from_xyz(
                        level.world_x as f32 / size.x as f32,
                        0.0,
                        level.world_y as f32 / size.y as f32,
                    ),
                    // visibility: Visibility::Visible,
                    ..default()
                },
                Name::new(format!("Tilemap #{}", uid)),
            ))
            .id();

        commands.send_event(SpawnMeshEvent { tilemap });
    }
}

#[derive(Debug, Reflect, Component, Default, Clone, Deref, DerefMut)]
pub struct TileDepth(pub i64);

/// Indicates the tile to use as reference for the slope height.
/// `TilePos` is the positioning of the tile *relative* to the current tile.
///
/// If TilePos is 0,0, then there is no slope here.
#[derive(Debug, Reflect, Component, Default, Clone)]
pub struct TileSlope(pub Vec2);

#[derive(Debug, Event, Reflect, Clone)]
pub struct SpawnMeshEvent {
    pub tilemap: Entity,
}

pub type TileDepthMapSerialization = HashMap<[u32; 2], i64>;
pub type TileSlopeMapSerialization = HashMap<[u32; 2], Vec2>;
