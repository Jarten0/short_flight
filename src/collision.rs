use bevy::prelude::*;
use bitflags::bitflags;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CollisionEvent>().add_systems(
            FixedPostUpdate,
            (
                // run GlobalTransform synchronization here for query_overlaps
                (
                    bevy::transform::systems::propagate_transforms,
                    bevy::transform::systems::sync_simple_transforms,
                ),
                query_overlaps, // this uses the latest GlobalTransform, but doesn't change any Transform's
                propogate_collision_events, // this will be changing Transform's after GlobalTransform is used
                                            // GlobalTransforms will be finally updated in PostUpdate
            )
                .chain(),
        );
    }
}

/// Declares the height of the object as a range in the 2d collision system.
///
/// This effectively means that hitboxes can't vary in shape depending on height,
/// but that's okay here because it's easier to read for top down gameplay.
#[derive(Debug, Component, Default)]
pub struct ZHitbox {
    /// If anything is equal to, or below this value and above neg_y_tolerance, it may be collided with.
    pub y_tolerance: f32,
    /// If anything is equal to, or above this value and below y_tolerance, it may be collided with.
    ///
    /// Should always be negative, unless trying to offset the ZHitbox from the Z transform
    pub neg_y_tolerance: f32,
}

/// Information for objects that move
#[derive(Debug, Reflect, Component, Default)]
pub struct DynamicCollision {}

/// Information for objects that cannot nor should ever move
#[derive(Debug, Reflect, Component, Default)]
pub struct StaticCollision {}

#[derive(Debug, Reflect, Component)]
#[require(ZHitbox, Transform)]
pub struct Collider {
    pub dynamic: bool,
    pub shape: ColliderShape,
    pub layers: CollisionLayers,
    pub can_interact: CollisionLayers,
}

#[derive(Debug, Clone, Reflect)]
pub struct CollisionLayers(u32);

bitflags! {
    impl CollisionLayers: u32 {
        const None = 0b00000000;
        const Default = 0b00000001;
        const Wall = 0b00000010;
        const NPC = 0b00000100;
        const Projectile = 0b00001000;
    }
}

impl Default for CollisionLayers {
    fn default() -> Self {
        Self::Default
    }
}

/// The different ways a shape can be represented - and *calculated* - for collision checks.
///
/// These operate in a faux-3d space with the z-position
///
/// Variance exists for the sake of performant collision checking. Most shapes can effectively be represented by a mesh.
/// But since it's quite the expensive operation to check for, its better to use other means where possible.
/// The difficulty comes in getting all of these different objects to work with each other.
///
/// Every added variant to Collider shape will need to have interoperability with all of the other types,
/// so being conservative with new additions where possible is ideal.
///
/// It's also ideal if this is not directly accessed by user code, for that reason.
#[derive(Debug, Reflect)]
pub enum ColliderShape {
    /// A rectangle, second easiest to calculate collisions with. Useful for bounding boxes.
    ///
    /// Transform rotation will *not* apply. it is always axis-aligned for ease of collision checking.
    Rect(Rect),
    /// The easiest shape to calculate collisions with, is useful for general object shapes.
    Circle { radius: f32 },
    /// the expensive but most customizable option, and the only one that makes use of the asset system
    Mesh(Handle<Mesh>),
}

#[derive(Debug, Event, Clone)]
pub struct CollisionEvent {
    pub this: Entity,
    pub other: Entity,
}

fn query_overlaps(
    dyn_objects: Query<(
        Entity,
        &GlobalTransform,
        &Collider,
        &ZHitbox,
        &DynamicCollision,
    )>,
    all_objects: Query<(
        Entity,
        &GlobalTransform,
        &Collider,
        &ZHitbox,
        AnyOf<(&DynamicCollision, &StaticCollision)>,
    )>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (entity, transform, dyn_col, z_hitbox, dyn_info) in &dyn_objects {
        match &dyn_col.shape {
            ColliderShape::Rect(col_rect) => {
                let rect = offset_rect(col_rect, transform);
                for (entity2, transform2, col, z_hitbox2, (dyn_info2, stat_info2)) in &all_objects {
                    if !dyn_col.can_interact.intersects(col.layers.clone()) {
                        continue;
                    }
                    if !z_intersecting((z_hitbox, transform), (z_hitbox2, transform2)) {
                        continue;
                    }
                    let result: bool = match &col.shape {
                        ColliderShape::Rect(col_rect) => {
                            let rect2 = offset_rect(col_rect, transform2);
                            !rect.intersect(rect2).is_empty()
                        }
                        ColliderShape::Circle { radius } => {
                            let p = transform2.translation().xz();

                            let radius2 = bounded_magnitude_squared(rect, p);

                            let distance_sq = rect.center().distance_squared(p);
                            radius.powi(2) + radius2 > distance_sq
                        }
                        ColliderShape::Mesh(handle) => unimplemented!(),
                    };

                    if result {
                        collision_events.send(CollisionEvent {
                            this: entity,
                            other: entity2,
                        });
                    }
                }
            }
            ColliderShape::Circle { radius } => {
                let p = transform.translation().xz();

                for (entity2, transform2, col, z_hitbox2, (dyn_info2, stat_info2)) in &all_objects {
                    if !z_intersecting((z_hitbox, transform), (z_hitbox2, transform2)) {
                        continue;
                    }
                    let result: bool = match &col.shape {
                        ColliderShape::Rect(col_rect) => {
                            let rect = offset_rect(col_rect, transform2);

                            let radius2 = bounded_magnitude_squared(rect, p);

                            let distance_sq = rect.center().distance_squared(p);

                            radius.powi(2) + radius2 > distance_sq
                        }
                        ColliderShape::Circle { radius: radius2 } => {
                            let p2 = transform2.translation().xz();

                            let distance_sq = p.distance_squared(p2);
                            radius.powi(2) + radius2.powi(2) > distance_sq
                        }
                        ColliderShape::Mesh(handle) => unimplemented!(),
                    };

                    if result {
                        collision_events.send(CollisionEvent {
                            this: entity,
                            other: entity2,
                        });
                    }
                }
            }
            ColliderShape::Mesh(handle) => unimplemented!(),
        }
    }
}

/// the magnitude of a vector pointing from `rect`'s center, towards `p`,
/// bounded by the `rect`, and squared for fast computing.
/// effectively it just turns the `rect` into a circle with the radius of the closest possible tangential point.
fn bounded_magnitude_squared(rect: Rect, p: Vec2) -> f32 {
    let difference = p - rect.center();
    let bounded = Vec2::new(
        difference.x.clamp(rect.min.x, rect.max.x),
        difference.y.clamp(rect.min.y, rect.max.y),
    );
    bounded.length_squared()
}

fn offset_rect(col_rect: &Rect, transform: &GlobalTransform) -> Rect {
    let mut rect = col_rect.clone();
    rect.min += transform.translation().xy();
    rect.max += transform.translation().xy();
    rect
}

// returns true if the two ZHitboxes are intersecting, even if their transform x's and y's are distant.
fn z_intersecting(zhb: (&ZHitbox, &GlobalTransform), rhzhb: (&ZHitbox, &GlobalTransform)) -> bool {
    let range = [
        zhb.0.neg_y_tolerance + zhb.1.translation().y,
        zhb.0.y_tolerance + zhb.1.translation().y,
    ];
    let range2 = [
        rhzhb.0.neg_y_tolerance + rhzhb.1.translation().y,
        rhzhb.0.y_tolerance + rhzhb.1.translation().y,
    ];
    let bottom_bound = (range[1] > range2[0]) | (range2[0] > range[1]);
    let top_bound = (range[0] < range2[1]) | (range2[1] < range[0]);
    bottom_bound & top_bound
}

fn propogate_collision_events(mut events: EventReader<CollisionEvent>, mut commands: Commands) {
    for event in events.read() {
        commands.trigger(event.clone());
    }
    if !events.is_empty() {
        events.clear();
    }
}
