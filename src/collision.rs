use bevy::prelude::*;

/// Collider that notates that the entity should interact and collide with the custom tilemaps given.
#[derive(Debug, Component, Reflect, Default)]
pub struct TilemapCollider {
    previous_pos: Vec2,
}

pub fn process_tilemap_collisions() {}
