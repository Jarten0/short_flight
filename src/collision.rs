use bevy::color::palettes;
use bevy::prelude::*;
use bevy::utils::hashbrown::{HashMap, HashSet};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

pub mod physics;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CollisionEnterEvent>()
            .add_event::<CollisionExitEvent>()
            .init_resource::<CollisionTracker>()
            .register_type::<BasicCollider>()
            .add_systems(FixedFirst, physics::update_dynamic_collision)
            .add_systems(
                FixedPostUpdate,
                (
                    physics::update_rigidbodies,
                    // run GlobalTransform synchronization here since query_overlaps needs
                    (
                        bevy::transform::systems::propagate_transforms,
                        bevy::transform::systems::sync_simple_transforms,
                    ),
                    query_overlaps, // this uses the latest GlobalTransform, but doesn't change any Transform's
                    // (
                    //     bevy::transform::systems::propagate_transforms,
                    //     bevy::transform::systems::sync_simple_transforms,
                    // ),
                    process_collisions_and_send_events,
                    propogate_collision_events, // this will be changing Transform's after GlobalTransform is used
                    cleanup_collision_tracker,
                    // GlobalTransforms will be finally updated in PostUpdate
                )
                    .chain(),
            );
    }
}

#[derive(Debug, Resource, Clone, Default, Serialize)]
pub struct CollisionTracker {
    pub current_collisions: HashMap<Entity, HashSet<(Entity, bool)>>,
}

/// Information for objects that move
#[derive(Debug, Reflect, Component, Default)]
pub struct DynamicCollision {
    pub previous_position: Vec3,
}

/// Information for objects that cannot nor should ever move
#[derive(Debug, Reflect, Component, Default)]
pub struct StaticCollision {}

#[derive(Debug, Reflect, Component, Clone, Serialize, Deserialize)]
#[require(ZHitbox, Transform)]
pub struct BasicCollider {
    pub dynamic: bool,
    pub shape: ColliderShape,
    pub layers: CollisionLayers,
    pub can_interact: CollisionLayers,
    /// Only stored if
    pub currently_colliding: HashSet<Entity>,
}

impl BasicCollider {
    pub fn new(
        dynamic: bool,
        shape: ColliderShape,
        layers: CollisionLayers,
        can_interact: CollisionLayers,
    ) -> Self {
        Self {
            dynamic,
            shape,
            layers,
            can_interact,
            currently_colliding: HashSet::new(),
        }
    }
}

/// Flags:
/// * Default    = 0b00000001;
/// * Wall       = 0b00000010;
/// * NPC        = 0b00000100;
/// * Projectile = 0b00001000;
/// * Attack     = 0b00010000;
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
// #[serde(transparent)]
pub struct CollisionLayers(u32);

bitflags! {
    impl CollisionLayers: u32 {
        const None = 0b00000000;
        const Default = 0b00000001;
        const Wall = 0b00000010;
        const NPC = 0b00000100;
        const Projectile = 0b00001000;
        const Attack = 0b00010000;
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
/// But it's quite the expensive operation to check for, so its better to use other means where possible.
///
/// Of course, things aren't always easy.
/// The difficulty comes in when trying to get all of these different shapes to work with each other.
///
/// Every added variant to Collider shape will need to have interoperability with all of the other variants,
/// so being conservative with new additions where possible is ideal.
#[derive(Debug, Reflect, Serialize, Deserialize, Clone)]
pub enum ColliderShape {
    /// A rectangle, second easiest to calculate collisions with. Useful for bounding boxes.
    ///
    /// Transform rotation will *not* apply. it is always axis-aligned for ease of collision checking.
    Rect(Rect),
    /// The easiest shape to calculate collisions with, is useful for general object shapes.
    Circle(f32),
    /// the expensive but most customizable option
    Mesh(Vec<Vec2>),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Circle(0.0)
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

impl ZHitbox {
    #[inline]
    pub fn height(&self) -> f32 {
        self.y_tolerance + self.neg_y_tolerance
    }

    /// Returns `true` if the two ZHitboxes are intersecting, even if their transform x's and y's are distant.
    pub fn intersecting(&self, other: &Self, self_y_offset: f32, other_y_offset: f32) -> bool {
        let range = [
            self.neg_y_tolerance + self_y_offset,
            self.y_tolerance + self_y_offset,
        ];
        let range2 = [
            other.neg_y_tolerance + other_y_offset,
            other.y_tolerance + other_y_offset,
        ];
        let bottom_bound = (range[1] >= range2[0]) & (range2[1] >= range[0]);
        let top_bound = (range[0] <= range2[1]) & (range2[0] <= range[1]);
        bottom_bound & top_bound
    }
}

#[derive(Debug, Event, Clone)]
pub struct CollisionEnterEvent {
    pub this: Entity,
    pub other: Entity,
}

#[derive(Debug, Event, Clone)]
pub struct CollisionExitEvent {
    pub this: Entity,
    pub other: Entity,
}

pub fn query_overlaps(
    dyn_objects: Query<(
        Entity,
        &GlobalTransform,
        &BasicCollider,
        &ZHitbox,
        &DynamicCollision,
    )>,
    all_objects: Query<(
        Entity,
        &GlobalTransform,
        &BasicCollider,
        &ZHitbox,
        AnyOf<(&DynamicCollision, &StaticCollision)>,
    )>,
    mut gizmos: Gizmos,
    mut collision_tracker: ResMut<CollisionTracker>,
) {
    for (entity, transform, dyn_col, z_hitbox, dyn_info) in &dyn_objects {
        let colliding: &mut HashSet<(Entity, bool)> =
            match collision_tracker.current_collisions.get_mut(&entity) {
                Some(some) => some,
                None => {
                    collision_tracker
                        .current_collisions
                        .insert(entity, HashSet::new());
                    collision_tracker
                        .current_collisions
                        .get_mut(&entity)
                        .unwrap()
                }
            };

        match &dyn_col.shape {
            ColliderShape::Rect(col_rect) => {
                let rect = offset_rect(col_rect, transform);
                for (entity2, transform2, col, z_hitbox2, (dyn_info2, stat_info2)) in &all_objects {
                    if entity == entity2 {
                        continue;
                    }

                    if !dyn_col.can_interact.intersects(col.layers.clone()) {
                        continue;
                    }

                    let mut result: bool = match &col.shape {
                        ColliderShape::Rect(col_rect) => {
                            let rect2 = offset_rect(col_rect, transform2);
                            !rect.intersect(rect2).is_empty()
                        }
                        ColliderShape::Circle(radius) => {
                            let p = transform2.translation().xz();

                            let radius2 = bounded_difference(rect, p).length_squared();

                            let distance_sq = rect.center().distance_squared(p);

                            radius.powi(2) + radius2 > distance_sq
                        }
                        ColliderShape::Mesh(handle) => unimplemented!(),
                    };

                    result &= z_hitbox.intersecting(
                        z_hitbox2,
                        transform.translation().y,
                        transform2.translation().y,
                    );

                    colliding.insert((entity2, result));
                }
            }
            ColliderShape::Circle(radius) => {
                let p = transform.translation().xz();

                const DRAW_GIZMOS: bool = false;

                for (entity2, transform2, col, z_hitbox2, (dyn_info2, stat_info2)) in &all_objects {
                    if entity == entity2 {
                        continue;
                    }

                    let mut result: bool = match &col.shape {
                        ColliderShape::Rect(col_rect) => {
                            let rect = offset_rect(col_rect, transform2);

                            let radius2 = bounded_difference(rect, p).length();

                            let combined_radius = radius + radius2;

                            let distance = rect.center().distance(p);

                            if DRAW_GIZMOS && combined_radius > distance - 0.5 {
                                gizmos.circle(
                                    Isometry3d::new(
                                        Vec3::new(rect.center().x, 0.05, rect.center().y),
                                        Quat::from_rotation_x(f32::to_radians(90.0)),
                                    ),
                                    radius2,
                                    palettes::basic::RED,
                                );
                            }

                            if combined_radius > distance {
                                if DRAW_GIZMOS {
                                    let end = Vec3::new(
                                        rect.center().x,
                                        1.0 + transform2.translation().y + z_hitbox2.y_tolerance,
                                        rect.center().y,
                                    );
                                    gizmos.line(
                                        Vec3::new(p.x, 0.1, p.y),
                                        end,
                                        palettes::basic::LIME,
                                    );
                                }

                                true
                            } else {
                                false
                            }
                        }
                        ColliderShape::Circle(radius2) => {
                            let p2 = transform2.translation().xz();

                            let distance_sq = p.distance_squared(p2);
                            radius.powi(2) + radius2.powi(2) > distance_sq
                        }
                        ColliderShape::Mesh(handle) => unimplemented!(),
                    };

                    result &= z_hitbox.intersecting(
                        z_hitbox2,
                        transform.translation().y,
                        transform2.translation().y,
                    );

                    colliding.insert((entity2, result));
                }
            }
            ColliderShape::Mesh(handle) => unimplemented!(),
        }
    }
}

pub fn process_collisions_and_send_events(
    collision_tracker: Res<CollisionTracker>,
    mut commands: Commands,
) {
    for (entity, colliding) in &collision_tracker.current_collisions {
        let entity = *entity;
        for (entity2, result) in colliding {
            let entity2 = *entity2;
            if *result {
                commands
                    .entity(entity)
                    .trigger(CollisionEnterEvent {
                        this: entity,
                        other: entity2,
                    })
                    .queue(DeferColliderUpdate {
                        enter: true,
                        other: entity2,
                    });
            } else {
                commands
                    .entity(entity)
                    .trigger(CollisionExitEvent {
                        this: entity,
                        other: entity2,
                    })
                    .queue(DeferColliderUpdate {
                        enter: false,
                        other: entity2,
                    });
            }
        }
    }
}

/// Deferred functionality for removing any collision trackings for collision exits
pub fn cleanup_collision_tracker(mut collision_tracker: ResMut<CollisionTracker>) {
    for (_, colliding) in &mut collision_tracker.current_collisions {
        colliding.clear();
    }
}

/// the magnitude of a vector pointing from `rect`'s center, towards `p`,
/// bounded by the `rect`, and squared for fast computing.
/// effectively it just turns the `rect` into a circle with the radius of the closest possible tangential point.
fn bounded_difference(rect: Rect, p: Vec2) -> Vec2 {
    let difference = p - rect.center();
    let min = rect.min - rect.center();
    let max = rect.max - rect.center();
    Vec2::new(
        difference.x.clamp(min.x, max.x),
        difference.y.clamp(min.y, max.y),
    )
}

fn offset_rect(col_rect: &Rect, transform: &GlobalTransform) -> Rect {
    let mut rect = col_rect.clone();
    rect.min += transform.translation().xz();
    rect.max += transform.translation().xz();
    rect
}

pub fn propogate_collision_events(
    mut events: EventReader<CollisionEnterEvent>,
    mut events2: EventReader<CollisionExitEvent>,
    mut commands: Commands,
) {
    for event in events.read() {
        commands.trigger(event.clone());
    }
    if !events.is_empty() {
        events.clear();
    }
    for event in events2.read() {
        commands.trigger(event.clone());
    }
    if !events2.is_empty() {
        events2.clear();
    }
}

struct DeferColliderUpdate {
    enter: bool,
    other: Entity,
}

impl EntityCommand for DeferColliderUpdate {
    fn apply(self, entity: Entity, world: &mut World) {
        if self.enter {
            world
                .get_mut::<BasicCollider>(entity)
                .unwrap()
                .currently_colliding
                .insert(self.other);
        } else {
            world
                .get_mut::<BasicCollider>(entity)
                .unwrap()
                .currently_colliding
                .remove(&self.other);
        }
    }
}
