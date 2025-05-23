use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

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

/// Unique representation of how a tile should be sloped.
///
/// `x` and `z` determine how much a tile should be sloped on either axis.
/// The higher the value, the higher that part of the slope will be.
///
/// `y` is used to determine the offset of the points from the initial depth of the tile.
/// See: [`TileDepth`], which is used to determine the base height of a tile.
///
/// [`TileFlags`] is used in tandem with this in order to determine several specifics about the slope,
/// notably [`TileFlags::Exclusive`], which needs some explaining, detailed below.
///
/// An inclusive tile - as according to my own definition - is a concave corner tile, or on the intersection of two walls.
/// It is also any "straight" slope, or any slope not a part of a corner and instead just part of a wall.
/// When a tile is in inclusive mode, its slope is calculated to have `x` and `z` add together when determining the height of the corner,
/// thus "including" inputs when both are present to get the final height of a corner.
///
/// An exclusive tile is the opposite. This is used specifically for convex corner tiles which need some annoyingly specific calculations.
/// When a tile is in exclusive mode, each corner is calculated
#[derive(Debug, Reflect, Component, Default, Clone, Serialize, Deserialize, Deref)]
#[serde(transparent)]
pub struct TileSlope(pub Vec3);

impl TileSlope {
    /// Returns the height of the slope at any given point.
    pub fn get_height_at_point(&self, tile_flags: &TileFlags, point: Vec2) -> f32 {
        // [tl, tr, br, bl]
        let corners = self.get_slope_corner_depths(!tile_flags.intersects(TileFlags::Exclusive));

        let triangle_1 = if !tile_flags.intersects(TileFlags::FlipTriangles) {
            [0, 1, 2]
        } else {
            [1, 0, 3]
        };
        let triangle_2 = if !tile_flags.intersects(TileFlags::FlipTriangles) {
            [0, 1, 2]
        } else {
            [1, 0, 3]
        };

        let y = Self::get_y_position_from_point_on_triangle(
            Vec3::new(0., corners[0], 0.),
            Vec3::new(1., corners[1], 0.),
            Vec3::new(1., corners[2], 1.),
            point.clamp(Vec2::ZERO, Vec2::ONE),
        );
        let y2 = Self::get_y_position_from_point_on_triangle(
            Vec3::new(0., corners[0], 0.),
            Vec3::new(1., corners[2], 1.),
            Vec3::new(0., corners[3], 1.),
            point.clamp(Vec2::ZERO, Vec2::ONE),
        );

        f32::max(y, y2)
    }

    /// Gets the maximum height of the slope for collision detection.
    /// Output is relative to base tile height.
    pub fn get_slope_height(&self, flags: &TileFlags) -> f32 {
        self.get_slope_corner_depths(!flags.intersects(TileFlags::Exclusive))
            .into_iter()
            .reduce(f32::max)
            .unwrap()
    }

    /// For the given slope (`s`) value, returns the depth of the four corners of a tile
    /// that should become a slope.
    ///
    /// `inclusive` determines the shape a slope will take,
    /// notably determining whether corner slopes become extrusive or intrusive.
    ///
    /// Returns the points in order of:
    ///
    /// 0. Top Left
    /// 1. Top Right
    /// 2. Bottom Right
    /// 3. Bottom Left
    pub(crate) fn get_slope_corner_depths(&self, inclusive: bool) -> [f32; 4] {
        let pos_x = self.x;
        let pos_z = self.z;
        let neg_x = -self.x;
        let neg_z = -self.z;

        let east = pos_x.clamp(0.0, f32::INFINITY);
        let west = neg_x.clamp(0.0, f32::INFINITY);
        let south = pos_z.clamp(0.0, f32::INFINITY);
        let north = neg_z.clamp(0.0, f32::INFINITY);

        let op = if inclusive { f32::max } else { f32::min };

        let tl = op(north, west);
        let tr = op(north, east);
        let br = op(south, east);
        let bl = op(south, west);

        [tl + self.y, tr + self.y, br + self.y, bl + self.y]
    }

    /// a, b, c : triangle
    /// point: relative position from the center of the tile
    fn get_y_position_from_point_on_triangle(a: Vec3, b: Vec3, c: Vec3, point: Vec2) -> f32 {
        let vector_one = b - a;
        let vector_two = c - a;
        let normal = vector_one.cross(vector_two);

        let (x, z) = point.into();
        let n = normal;

        let numerator = -((n.x * (x - a.x)) + (n.z * (z - a.z)));
        let denominator = n.y;

        // log::info!("point:{} a:{} normal:{}", point, a, normal);
        // log::info!(
        //     "num,den:{:?} result:{}",
        //     (numerator, denominator),
        //     numerator / denominator
        // );
        assert_ne!(numerator / denominator, f32::NEG_INFINITY);
        assert_ne!(numerator / denominator, f32::INFINITY);
        assert_ne!(numerator / denominator, f32::NAN);

        a.y + (numerator / denominator)
    }
}

/// Bitflags for how the tile should be visibly changed
///
/// Flags:
/// * FlipX = 0b1;
/// * FlipY = 0b1 << 1;
/// * RotateClockwise = 0b1 << 2;
/// * RotateCounterClockwise = 0b1 << 3; **May become obsolete in the future**
/// * FlipTriangles = 0b1 << 4;
/// * Exclusive = 0b1 << 5;
/// * Fold = 0b1 << 6;
///
/// See per-flag documentation for more details.
#[derive(Debug, Reflect, Component, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct TileFlags(u32);

bitflags! {
    impl TileFlags: u32 {
        /// Flip the texture of the tile across the x-axis.
        const FlipX = 0b1;
        /// Flip the texture of the tile across the y-axis.
        const FlipY = 0b1 << 1;
        /// Rotates the texture by 90 degrees clockwise.
        ///
        /// Applies after texture flips, not before.
        ///
        /// Combine `FlipX` and `FlipY` for a 180 degree rotation.
        const RotateClockwise = 0b1 << 2;
        /// Rotates the texture by 90 degrees counterclockwise.
        ///
        /// **NOTE:** Might not be a necessary flag, may become obsolete/changed if another flag proves more useful.
        ///
        /// Applies after texture flips, not before.
        ///
        /// Combine `FlipX` and `FlipY` for a 180 degree rotation.
        const RotateCounterClockwise = 0b1 << 3;
        /// Changes the vertex calculations so that the top tile face's triangles will use an alternative layout.
        /// Effect is important for corner slopes, which change based on the orientation of the triangles.
        const FlipTriangles = 0b1 << 4;
        /// Changes slope calculations to use an alternate method of calculating corners from the [`TileSlope`].0's [`Vec3`] values.
        /// See [`TileSlope`] for more details.
        const Exclusive = 0b1 << 5;
        /// Rotates one of the triangle's textures, which will allow for more consistent
        /// corner slopes lining up with alternatively rotated sides.
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
