use std::marker::PhantomData;

use bevy::color::palettes;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::assets::{self, ShortFlightLoadingState};
use crate::ldtk::TileQuery;
use crate::tile::{TileDepth, TileFlags, TileSlope};

pub mod physics;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CollisionEnterEvent>()
            .add_event::<CollisionExitEvent>()
            .init_resource::<CollisionTracker>()
            .init_resource::<CollisionTracker<StaticCollision>>()
            .init_resource::<CollisionTracker<TilemapCollision>>()
            .register_type::<BasicCollider>()
            .add_systems(FixedFirst, physics::update_dynamic_collision)
            .add_systems(
                FixedPostUpdate,
                (
                    physics::update_rigidbodies,
                    // run GlobalTransform synchronization here since query_overlaps needs
                    (
                        bevy::transform::systems::propagate_parent_transforms,
                        bevy::transform::systems::sync_simple_transforms,
                    ),
                    // Parallel collision tracking done here
                    (query_collider_overlaps, query_tile_overlaps), // these use the latest GlobalTransform, but doesn't change any Transform's
                    // Synchronize parallelized collision trackers here
                    |mut main_source: ResMut<CollisionTracker>,
                     mut collider: ResMut<CollisionTracker<StaticCollision>>,
                     mut tile: ResMut<CollisionTracker<TilemapCollision>>| {
                        main_source.sync(&mut collider);
                        main_source.sync(&mut tile);
                    },
                    // (
                    //     bevy::transform::systems::propagate_parent_transforms,
                    //     bevy::transform::systems::sync_simple_transforms,
                    // ),
                    process_collisions_and_send_events,
                    propogate_collision_events, // this will be changing Transform's after GlobalTransform is used
                    // Deferred functionality for removing any collision trackings for collision exits
                    |mut collision_tracker: ResMut<CollisionTracker>| {
                        collision_tracker.cleanup();
                    },
                    // GlobalTransforms will be finally updated in PostUpdate
                )
                    .chain()
                    .run_if(assets::loaded),
            );
    }
}

/// Parallelizable tracker for new collision events.
///
/// When a collision event for a particular entity occurs, it should be registered here.
/// Use different generic types to instantiate multiple versions of CollisionTracker.
///
/// Later on, all collision trackers should be syncronized in
#[derive(Debug, Resource, Clone, Serialize)]
pub struct CollisionTracker<T = ()> {
    /// Tracks events that need to be triggered for entities.
    event_trackers: HashMap<Entity, CollisionEventTracker>,
    /// Marker used so that multiple instantiations of CollisionTracker may be inserted into a single world
    /// and thus may be used in parallel, synchronizing later into one main instance.
    ///  
    /// The unit type [ () ] is used to indicate that this CollisionTracker is the main source of information
    marker: PhantomData<T>,
}

impl CollisionTracker {
    /// Returns the list of event trackers
    pub fn current_collisions(&self) -> &HashMap<Entity, CollisionEventTracker> {
        &self.event_trackers
    }

    /// Resets registered event trackers.
    pub fn cleanup(&mut self) {
        for (_entity, event_tracker) in &mut self.event_trackers {
            event_tracker.clear();
        }
    }

    /// Appends event trackers to get one cohesive iterable list.
    fn sync<T>(&mut self, other: &mut CollisionTracker<T>) {
        self.event_trackers = self
            .event_trackers
            .clone()
            .into_iter()
            .chain(other.event_trackers.drain())
            .collect();
    }
}

impl<T> Default for CollisionTracker<T> {
    fn default() -> Self {
        Self {
            event_trackers: Default::default(),
            marker: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CollisionEventTracker(HashMap<Entity, bool>);

impl CollisionEventTracker {
    /// Registers a new event.
    ///
    /// Note: This should only be ran **once** per overlap/unoverlap, right when it happens.
    /// If it was already registered, then do not call again until the overlap state changes again.
    pub fn register_overlap(&mut self, entity: Entity, is_overlapping: bool) {
        self.0.insert(entity, is_overlapping);
    }

    /// Clears the event list.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Labeller for collision entities that move around.
#[derive(Debug, Reflect, Component, Default)]
pub struct DynamicCollision;

/// Labeller for collision entities that should interact with tilemaps
#[derive(Debug, Component)]
pub struct TilemapCollision;

/// Information for objects that cannot nor should ever move
#[derive(Debug, Reflect, Component, Default)]
pub struct StaticCollision;

#[derive(Debug, Reflect, Component, Clone, Serialize, Deserialize)]
#[require(ZHitbox, Transform)]
pub struct BasicCollider {
    pub dynamic: bool,
    pub shape: ColliderShape,
    pub layers: CollisionLayers,
    pub can_interact: CollisionLayers,
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
    pub const fn height(&self) -> f32 {
        self.y_tolerance + self.neg_y_tolerance
    }

    /// Returns `true` if the two ZHitboxes are intersecting, even if their transform x's and y's are distant.
    pub const fn intersecting(
        &self,
        other: &Self,
        self_y_offset: f32,
        other_y_offset: f32,
    ) -> bool {
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

/// For dynamic collider entities, finds all overlaps with other collider entities.
pub fn query_collider_overlaps(
    dyn_objects: Query<(Entity, &GlobalTransform, &BasicCollider), With<DynamicCollision>>,
    all_objects: Query<
        (Entity, &GlobalTransform, &BasicCollider),
        Or<(With<DynamicCollision>, With<StaticCollision>)>,
    >,
    z_query: Query<(&ZHitbox, &GlobalTransform)>,
    mut collision_tracker: ResMut<CollisionTracker>,
) {
    for (entity, transform, dyn_col) in &dyn_objects {
        let event_tracker = collision_tracker
            .event_trackers
            .entry(entity)
            .or_insert(Default::default());

        let entity2_query = all_objects
            .iter()
            .filter(|(entity2, _, _)| entity != *entity2)
            .filter(|(_, _, col)| dyn_col.can_interact.intersects(col.layers.clone()));

        let overlap_results: HashMap<Entity, bool> = match &dyn_col.shape {
            ColliderShape::Rect(col_rect) => {
                let rect = offset_rect(col_rect, transform);
                entity2_query
                    .map(|(entity2, transform2, col)| {
                        (entity2, get_rect_overlaps(rect, transform2, col))
                    })
                    .collect()
            }
            ColliderShape::Circle(radius) => {
                let p = transform.translation().xz();

                entity2_query
                    .map(|(entity2, transform2, col)| {
                        (entity2, get_circle_overlaps(radius, p, transform2, col))
                    })
                    .collect()
            }
            ColliderShape::Mesh(handle) => unimplemented!(),
        };

        overlap_results
            .into_iter()
            .for_each(|(entity2, is_overlapping)| {
                if dyn_col.currently_colliding.contains(&entity2) == is_overlapping {
                    return;
                }
                if is_overlapping {
                    log::info!("{}", entity2);
                }
                event_tracker.register_overlap(
                    entity2,
                    is_overlapping && {
                        let (z1, g1) = z_query.get(entity).unwrap_or((
                            &ZHitbox {
                                y_tolerance: 0.0,
                                neg_y_tolerance: 0.0,
                            },
                            &GlobalTransform::IDENTITY,
                        ));
                        let (z2, g2) = z_query.get(entity2).unwrap_or((
                            &ZHitbox {
                                y_tolerance: 0.0,
                                neg_y_tolerance: 0.0,
                            },
                            &GlobalTransform::IDENTITY,
                        ));

                        z1.intersecting(z2, g1.translation().y, g2.translation().y)
                    },
                )
            })
    }
}

fn get_rect_overlaps(rect: Rect, transform2: &GlobalTransform, col: &BasicCollider) -> bool {
    match &col.shape {
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
    }
}

fn get_circle_overlaps(
    radius: &f32,
    p: Vec2,
    transform2: &GlobalTransform,
    col: &BasicCollider,
) -> bool {
    match &col.shape {
        ColliderShape::Rect(col_rect) => {
            let rect = offset_rect(col_rect, transform2);

            let radius2 = bounded_difference(rect, p).length();

            let combined_radius = radius + radius2;

            let distance = rect.center().distance(p);

            if combined_radius >= distance {
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
    }
}

/// For dynamic collider entities, finds all overlaps with colliding tiles.
///
/// Currently assumes all tiles go infinitely down.
/// TODO: patch in tile minimum cap
pub fn query_tile_overlaps(
    entities: Query<(Entity, &GlobalTransform, &ZHitbox, &BasicCollider), With<TilemapCollision>>,
    tile_query: TileQuery,
    tile_data: Query<(Entity, &TileDepth, &TileSlope, &TileFlags)>,
    mut gizmos: Gizmos,
    mut collision_tracker: ResMut<CollisionTracker<TilemapCollision>>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    let enable_gizmos = kb.pressed(KeyCode::KeyZ);
    for (entity, gtransform, zhitbox, basic_collider) in entities {
        let translation = gtransform.translation();
        let position = translation.xz();
        let mut overlapping_tiles: HashSet<Entity> = HashSet::new();

        match &basic_collider.shape {
            ColliderShape::Rect(rect) => {
                let rect_tile_overlap = IRect {
                    min: rect.min.floor().as_ivec2(),
                    max: rect.max.ceil().as_ivec2(),
                };

                let rect_tile_iter = (rect_tile_overlap.min.x..rect_tile_overlap.max.x)
                    .flat_map(|x| {
                        (rect_tile_overlap.min.y..rect_tile_overlap.max.y).map(move |y| (x, y))
                    })
                    .map(|(x, y)| Vec2 {
                        x: x as f32 + 0.5,
                        y: y as f32 + 0.5,
                    });

                overlapping_tiles = rect_tile_iter
                    .filter_map(|tile_pos| {
                        tile_query.get_tile(Vec3 {
                            x: tile_pos.x,
                            y: 0.0,
                            z: tile_pos.y,
                        })
                    })
                    .collect();
            }
            ColliderShape::Circle(radius) => {
                let tilemap_bounding_box = IRect {
                    min: (position - Vec2::splat(*radius)).floor().as_ivec2(),
                    max: (position + Vec2::splat(*radius)).ceil().as_ivec2(),
                };

                let bounding_box_tile_iter = (tilemap_bounding_box.min.x
                    ..tilemap_bounding_box.max.x)
                    .flat_map(|x| {
                        (tilemap_bounding_box.min.y..tilemap_bounding_box.max.y)
                            .map(move |y| (x, y))
                    })
                    .map(|(x, y)| Vec2 {
                        x: x as f32 + 0.5,
                        y: y as f32 + 0.5,
                    });

                for tile_pos in bounding_box_tile_iter.clone() {
                    if enable_gizmos {
                        gizmos.rect(
                            Isometry3d::new(
                                tile_pos.xxy().with_y(3.5),
                                Quat::from_rotation_x(f32::to_radians(90.)),
                            ),
                            Vec2::ONE,
                            palettes::basic::MAROON,
                        );
                    }

                    let f = |tile_pos: &Vec2| {
                        position.move_towards(*tile_pos, *radius + std::f32::consts::FRAC_1_SQRT_2)
                    };

                    let move_towards = f(&tile_pos);

                    // if !(move_towards == tile_pos) {
                    //     log::info!("{}", (move_towards - tile_pos).length())
                    // }

                    if enable_gizmos {
                        gizmos.line(
                            Vec3 {
                                x: tile_pos.x,
                                y: 2.,
                                z: tile_pos.y,
                            },
                            Vec3 {
                                x: move_towards.x,
                                y: 2.,
                                z: move_towards.y,
                            },
                            palettes::basic::AQUA,
                        );
                    }
                }

                overlapping_tiles = bounding_box_tile_iter
                    .filter(|tile_pos| {
                        let move_towards = position
                            .move_towards(*tile_pos, *radius + std::f32::consts::FRAC_1_SQRT_2);
                        return move_towards == *tile_pos;
                    })
                    .filter_map(|tile_pos| {
                        tile_query.get_tile(Vec3 {
                            x: tile_pos.x,
                            y: 0.0,
                            z: tile_pos.y,
                        })
                    })
                    .collect();
            }
            ColliderShape::Mesh(vec2s) => todo!(),
        }

        let overlapping_tile_query = overlapping_tiles
            .clone()
            .into_iter()
            .filter_map(|tile_entity| tile_data.get(tile_entity).ok());

        let event_tracker = collision_tracker
            .event_trackers
            .entry(entity)
            .or_insert(Default::default());

        for (tile_entity, depth, slope, flags) in overlapping_tile_query {
            if !zhitbox.intersecting(
                &ZHitbox {
                    y_tolerance: slope.get_slope_height(flags),
                    neg_y_tolerance: f32::NEG_INFINITY,
                },
                translation.y,
                depth.f32(),
            ) {
                continue;
            };

            if basic_collider.currently_colliding.contains(&tile_entity) {
                continue;
            }

            event_tracker.register_overlap(tile_entity, true);
        }

        for tile_entity in &basic_collider.currently_colliding {
            if !overlapping_tiles.contains(tile_entity) {
                event_tracker.register_overlap(*tile_entity, false);
                continue;
            }

            let Ok((tile_entity, depth, slope, flags)) =
                tile_data.get(*tile_entity).inspect_err(|err| {
                    log::error!(
                        "Could not query tile data for tile {}! [{}]",
                        tile_entity,
                        err
                    )
                })
            else {
                continue;
            };

            if !zhitbox.intersecting(
                &ZHitbox {
                    y_tolerance: slope.get_slope_height(flags),
                    neg_y_tolerance: f32::NEG_INFINITY,
                },
                translation.y,
                depth.f32(),
            ) {
                event_tracker.register_overlap(tile_entity, false);
            };
        }
    }
}
pub fn process_collisions_and_send_events(
    collision_tracker: Res<CollisionTracker>,
    mut commands: Commands,
) {
    for (entity, colliding) in collision_tracker.current_collisions() {
        let entity = *entity;
        for (entity2, result) in &colliding.0 {
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
    fn apply(self, mut entity: EntityWorldMut) -> () {
        let id = entity.id();
        entity.world_scope(|world| {
            if self.enter {
                world
                    .get_mut::<BasicCollider>(id)
                    .unwrap()
                    .currently_colliding
                    .insert(self.other);
            } else {
                world
                    .get_mut::<BasicCollider>(id)
                    .unwrap()
                    .currently_colliding
                    .remove(&self.other);
            }
        });
    }
}
