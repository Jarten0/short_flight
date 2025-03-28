use bevy::prelude::Asset;
use bevy::{asset::io::Reader, reflect::TypePath};
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext},
    prelude::*,
};
use bevy_ecs_tilemap::map::TilemapType;
use bevy_ecs_tilemap::tiles::TileFlip;
use bevy_ecs_tilemap::{
    map::{TilemapId, TilemapSize, TilemapTexture, TilemapTileSize},
    tiles::{TileBundle, TilePos, TileStorage, TileTextureIndex},
    TilemapBundle,
};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use short_flight::collision::{Collider, ColliderShape, CollisionLayers, StaticCollision, ZHitbox};
use short_flight::deserialize_file;
use std::{collections::HashMap, io::ErrorKind};
use thiserror::Error;

use crate::npc;
use crate::npc::{commands::SpawnNPC, NPC};

/// Initialized differently from the LDTK map data, this determines how high up the object is.
// There's no settlement on if the value will be represented as an `i64` in the future
// so for now, just use f32 and i64 to access the value, and From to set.
#[derive(Debug, Reflect, Component, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TileDepth(i64);

impl TileDepth {
    /// use this if
    #[inline]
    pub fn f32(&self) -> f32 {
        self.0 as f32
    }

    /// use this instead of accessing if potentially using an f32 converted to an i64 is future proof
    #[inline]
    pub fn i64(&self) -> i64 {
        self.0
    }
}

impl Into<f32> for TileDepth {
    fn into(self) -> f32 {
        self.0 as f32
    }
}

impl From<f32> for TileDepth {
    fn from(value: f32) -> Self {
        Self(value as i64)
    }
}
impl From<i64> for TileDepth {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Reflect, Component, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TileSlope(pub Vec3);

/// Bitflags for how the tile should be visibly changed
///
/// Rotation bitflags are assumed to be clockwise
#[derive(Debug, Reflect, Component, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct TileFlags(u32);

bitflags! {
    impl TileFlags: u32 {
        const FlipX = 0b1;
        const FlipY = 0b1 << 1;
        const RotateClockwise = 0b1 << 2;
        const RotateCounterClockwise = 0b1 << 3;
        const FlipTriangles = 0b1 << 4;
        const Exclusive = 0b1 << 5;
        const Fold = 0b1 << 6;
        // const  = 0b1 << 7;
    }
}

impl std::fmt::Display for TileFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flags = self
            .iter_names()
            .filter(|value| value.1.intersects(*self))
            .map(|value| value.0)
            .fold("".to_string(), |a, b| a + b + ", ");
        write!(f, "{}", flags)
    }
}

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
    let mut tile_depth_map: HashMap<[u32; 2], TileDepth> =
        deserialize_file("assets/depth_maps/tile_depth_map.ron").unwrap_or_default();
    let mut tile_slope_map: HashMap<[u32; 2], TileSlope> =
        deserialize_file("assets/depth_maps/tile_slope_map.ron").unwrap_or_default();
    let mut tile_flag_map: HashMap<[u32; 2], TileFlags> =
        deserialize_file("assets/depth_maps/tile_flag_map.ron").unwrap_or_default();

    log::info!("Found tilemap depth data of {} tiles", tile_depth_map.len());
    log::info!("Found tilemap slope data of {} tiles", tile_slope_map.len());
    log::info!("Found tilemap flag data of {} tiles", tile_flag_map.len());

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
        // Instantiate layer entities here
        for entity in layer.entity_instances.iter() {
            if entity.tags.contains(&"NPC".to_string()) {
                let id = entity
                    .field_instances
                    .iter()
                    .find(|field| field.identifier == "NPC_ID")
                    .unwrap()
                    .value
                    .as_ref()
                    .unwrap()
                    .as_u64()
                    .unwrap();

                commands.queue(npc::commands::SpawnNPC {
                    npc_id: NPC::try_from(id as usize).unwrap(),
                    position: Vec3::new(entity.px[0] as f32 / 32., 0.0, entity.px[1] as f32 / 32.),
                });

                let name = entity
                    .field_instances
                    .iter()
                    .find(|field| field.identifier == "Name")
                    .unwrap()
                    .value
                    .as_ref()
                    .unwrap()
                    .as_str()
                    .unwrap();

                log::info!("Spawned NPC: {}", name)
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

            let tile_flags = tile_flag_map.remove(&key).unwrap_or(
                TileFlags::default() | TileFlags::from_bits(tile.f as u32).unwrap_or_default(),
            );

            let bundle = (
                Name::new(format!("Tile {}-{:?}", layer_id, position)),
                position,
                TilemapId(map_entity),
                TileTextureIndex(tile.t as u32),
                Collider {
                    dynamic: false,
                    shape: ColliderShape::Rect(Rect {
                        min: Vec2 { x: 0., y: 0. },
                        max: Vec2 { x: 1., y: 1. },
                    }),
                    layers: CollisionLayers::Wall,
                    can_interact: CollisionLayers::all(),
                },
                ZHitbox {
                    y_tolerance: tile_slope.0.abs().max_element(),
                    neg_y_tolerance: f32::NEG_INFINITY,
                },
                StaticCollision {},
                tile_depth,
                tile_slope,
                tile_flags,
            );

            let tile_entity = commands.spawn(bundle).id();
            children.push(tile_entity.clone());
            storage.set(&position, tile_entity);
        }

        let tilemap = commands
            .entity(map_entity)
            .add_children(&children)
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
                    ..default()
                },
                Name::new(format!("Tilemap #{}", layer_id)),
            ))
            .id();

        commands.send_event(SpawnMeshEvent { tilemap });
    }
}
