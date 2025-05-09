use std::marker::PhantomData;

use bevy::color::palettes;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_ecs_tilemap::anchor::TilemapAnchor;
use bevy_ecs_tilemap::map::{TilemapGridSize, TilemapSize, TilemapTileSize, TilemapType};
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};

use crate::player::ClientQuery;

use super::tilemap_manager::{TilemapManager, minstep_to_tilemap};

pub fn get_tilemap_tilepos_from_world_position(
    world_position: Vec3,
    tilemap_manager: &TilemapManager,
) -> bevy::prelude::IVec2 {
    IVec2::new(
        minstep_to_tilemap(world_position.x.round() as i32) - tilemap_manager.offset.x,
        // minstep_to_tilemap(world_position.y.round() as i32) - tilemap_manager.offset,
        minstep_to_tilemap(world_position.z.round() as i32) - tilemap_manager.offset.y,
    )
}

#[derive(SystemParam)]
pub struct TileQuery<'w, 's> {
    pub tilemap_manager: Option<Res<'w, TilemapManager>>,
    pub query: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static TileStorage,
            &'static TilemapSize,
            &'static TilemapGridSize,
            &'static TilemapTileSize,
            &'static TilemapType,
            &'static TilemapAnchor,
        ),
    >,
}

impl TileQuery<'_, '_> {
    pub fn get_tile(&self, world_pos: Vec3) -> Option<Entity> {
        let Some(manager) = &self.tilemap_manager else {
            return None;
        };

        let tile_pos = get_tilemap_tilepos_from_world_position(world_pos, &manager);
        let tile_pos = TilePos::from_i32_pair(tile_pos.x, tile_pos.y, &manager.size)?;

        let entity = manager.map_storage.get(&tile_pos)?;

        let (tilemap_transform, storage, size, grid_size, tile_size, tilemap_type, tilemap_anchor) =
            self.query.get(entity).ok()?;

        let player_pos = world_pos.xz() - (Vec2::ONE / 2.);
        let vec2 = (player_pos - tilemap_transform.translation().xz()) * 32.;

        let tile_pos2 = TilePos::from_world_pos(
            &vec2,
            size,
            grid_size,
            tile_size,
            tilemap_type,
            tilemap_anchor,
        )?;

        storage.get(&tile_pos2)
    }
}

pub fn label_chunks(tile_query: TileQuery, client: ClientQuery<&Transform>, mut gizmos: Gizmos) {
    let TileQuery {
        tilemap_manager: Some(manager),
        query,
    } = tile_query
    else {
        return;
    };

    gizmos.cross(
        Isometry3d::from_translation(client.translation),
        0.25,
        palettes::basic::LIME,
    );

    gizmos.grid(
        Isometry3d::new(
            // Vec3::ZERO.with_y(2.),
            ((manager.offset.as_vec2() * 8.) + (Vec2::from(manager.size) / 2. * 8.))
                .xxy()
                .with_y(2.),
            // Quat::IDENTITY,
            Quat::from_rotation_x(f32::to_radians(90.)),
        ),
        manager.size.into(),
        Vec2::ONE * 8.,
        palettes::basic::RED,
    );

    let tile_pos = get_tilemap_tilepos_from_world_position(client.translation, &manager);

    gizmos.cross(
        Isometry3d::from_translation(
            ((tile_pos.as_vec2() * 8.) + (manager.offset.as_vec2() * 8.))
                .xyy()
                .with_y(3.),
        ),
        1.,
        palettes::basic::TEAL,
    );

    let Some(tile_pos) = TilePos::from_i32_pair(tile_pos.x, tile_pos.y, &manager.size) else {
        return;
    };

    let Some(entity) = manager.map_storage.get(&tile_pos) else {
        return;
    };

    let Some((tilemap_transform, storage, size, grid_size, tile_size, tilemap_type, anchor)) =
        query.get(entity).ok()
    else {
        return;
    };

    gizmos.cross(
        Isometry3d::from_translation(tilemap_transform.translation().with_y(1.)),
        size.x.min(size.y) as f32,
        palettes::basic::PURPLE,
    );

    let player_pos = client.translation.xz() - (Vec2::ONE / 2.);
    let vec2 = (player_pos - tilemap_transform.translation().xz()) * 32.;

    let Some(tile_pos2) =
        TilePos::from_world_pos(&vec2, size, grid_size, tile_size, &tilemap_type, &anchor)
    else {
        return;
    };

    gizmos.rect(
        Isometry3d::new(
            (Vec2::from(tile_pos2) + tilemap_transform.translation().xz() + (Vec2::ONE / 2.))
                .xxy()
                .with_y(4.),
            Quat::from_rotation_x(f32::to_radians(90.)),
        ),
        Vec2::ONE,
        palettes::basic::MAROON,
    );
}
