use bevy::prelude::*;

pub struct CollisionPlugin {}

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CollisionEvent>().add_systems(
            PostUpdate,
            ((query_overlaps),)
                .after(bevy::transform::plugins::TransformSystem::TransformPropagate)
                .chain(),
        );
    }
}

/// Information for objects that move
#[derive(Debug, Reflect, Component)]
pub struct DynamicCollision {
    previous_pos: Vec2,
}

/// Information for objects that cannot nor should ever move
#[derive(Debug, Reflect, Component)]
pub struct StaticCollision {}

#[derive(Debug, Reflect, Component)]
pub struct Collider {
    dynamic: bool,
    shape: ColliderShape,
}

#[derive(Debug, Reflect)]
pub enum ColliderShape {
    // all collision info about the tile can be queried for
    /// A 3D-ish tile, with a position, size, and depth.
    Tile,
    // transform rotation will *not* apply.
    Rect(Rect),
    Circle {
        radius: f32,
    },
}

#[derive(Debug, Event, Clone)]
pub struct CollisionEvent {}

fn query_overlaps(
    dyn_objects: Query<(&Collider, &DynamicCollision)>,
    all_objects: Query<(&Collider, AnyOf<(&DynamicCollision, &StaticCollision)>)>,
) {
    for (dyn_col, dyn_info) in &dyn_objects {
        for (col, (dyn_info, stat_info)) in &all_objects {
            todo!()
        }
    }
}
