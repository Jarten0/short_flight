use bevy::prelude::*;
use bevy_ecs_tilemap::map::{TilemapGridSize, TilemapSize, TilemapTileSize};
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};
use ldtk_rust::Level;

use crate::to_minsteps_i32;

#[derive(Debug, Resource)]
pub struct TilemapManager {
    // tilemap units
    pub map_storage: TileStorage,
    // tilemap units
    /// always negative
    pub offset: IVec2,
    // tilemap units
    pub size: TilemapSize,
    // tilemap units
    pub grid_size: TilemapGridSize,
    // tilemap units
    pub tile_size: TilemapTileSize,
}

/// takes a larger minstep unit and converts it to a smaller tilemap unit
pub fn minstep_to_tilemap(input: i32) -> i32 {
    input.div_floor(8)
}

impl TilemapManager {
    pub fn from_project(project: &ldtk_rust::Project) -> Self {
        let mut container: IRect = IRect::EMPTY;

        // size of a tilemap unit in minsteps
        let grid_size = IVec2 {
            x: to_minsteps_i32(project.world_grid_width.unwrap() as i32),
            y: to_minsteps_i32(project.world_grid_height.unwrap() as i32),
        };

        for level in &project.levels {
            // the offset of the level position in tilemap units
            // 1024px / 32px = 32mnst
            // 32mnst / 8mnst = 4tu
            let level_pos = IVec2 {
                x: minstep_to_tilemap(to_minsteps_i32(level.world_x as i32)),
                y: minstep_to_tilemap(to_minsteps_i32(level.world_y as i32)),
            };
            // the width and height of the level in tilemap units
            // 512px / 32px = 16mnst
            // 16mnst / 8mnst = 2tu
            let level_size = IVec2 {
                x: minstep_to_tilemap(to_minsteps_i32(level.px_wid as i32)),
                y: minstep_to_tilemap(to_minsteps_i32(level.px_hei as i32)),
            };
            let other = IRect::from_corners(level_pos, level_pos + level_size);

            container = container.union(other);
        }

        // tilemap units
        let size = TilemapSize::from(container.size().as_uvec2());
        // minstep units
        let tile_size = TilemapTileSize::from(grid_size.as_vec2());
        // minstep units
        let grid_size = TilemapGridSize::from(grid_size.as_vec2());
        // tilemap units
        let map_storage = TileStorage::empty(size);

        Self {
            map_storage,
            // tilemap units
            offset: container.min,
            size,
            grid_size,
            tile_size,
        }
    }

    pub fn insert(&mut self, level: &Level, entity: Entity, commands: &mut Commands) {
        let level_pos = IVec2 {
            x: minstep_to_tilemap(to_minsteps_i32(level.world_x as i32)),
            y: minstep_to_tilemap(to_minsteps_i32(level.world_y as i32)),
        };

        let tile_pos = TilePos::from((level_pos - self.offset).as_uvec2());
        log::info!("{:?}", tile_pos);
        let level_chunk_width = minstep_to_tilemap(to_minsteps_i32(level.px_wid as i32)) as u32;
        let level_chunk_height = minstep_to_tilemap(to_minsteps_i32(level.px_hei as i32)) as u32;
        for x in 0..level_chunk_width {
            for y in 0..level_chunk_height {
                let offset = TilePos::new(tile_pos.x + x, tile_pos.y + y);
                self.map_storage.set(&offset, entity);
            }
        }

        commands.entity(entity).insert(tile_pos);
    }
}
