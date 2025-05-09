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

#[derive(Debug, Reflect, Component, Default, Clone, Serialize, Deserialize, Deref)]
#[serde(transparent)]
pub struct TileSlope(pub Vec3);

impl TileSlope {
    pub fn get_height_at_point(&self, tile_flags: &TileFlags, point: Vec2) -> f32 {
        let corners = self.get_slope_corner_depths(!tile_flags.intersects(TileFlags::Exclusive));

        let y = Self::get_y_position_from_point_on_triangle(
            Vec3 {
                x: 0.,
                y: corners[0],
                z: 1.,
            },
            Vec3 {
                x: 1.,
                y: corners[1],
                z: 1.,
            },
            Vec3 {
                x: 1.,
                y: corners[2],
                z: 0.,
            },
            point,
        );
        let y2 = Self::get_y_position_from_point_on_triangle(
            Vec3 {
                x: 1.,
                y: corners[0],
                z: 1.,
            },
            Vec3 {
                x: 1.,
                y: corners[2],
                z: 0.,
            },
            Vec3 {
                x: 0.,
                y: corners[3],
                z: 0.,
            },
            point,
        );

        // log::info!("{:?}", (y, y2));
        let max = f32::max(y, y2);
        max
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
        let i = inclusive as i32 as f32;
        let pos: Box<dyn Fn(f32) -> f32> =
            Box::new(|value: f32| f32::clamp(value, 0., f32::INFINITY));
        let neg: Box<dyn Fn(f32) -> f32> =
            Box::new(|value: f32| -f32::clamp(value, f32::NEG_INFINITY, 0.));

        let points = [self.z; 4];
        let mapping = [
            (&neg, &pos), // tl
            (&pos, &pos), // tr
            (&pos, &neg), // br
            (&neg, &neg), // bl
        ];

        let select_corner =
            |(point, map): (f32, (&Box<dyn Fn(f32) -> f32>, &Box<dyn Fn(f32) -> f32>))| {
                let x_component = map.0(self.x);
                let z_component = map.1(self.z);

                let mut total = 0.;

                if x_component == 0. {
                    total += z_component * i
                } else if z_component == 0. {
                    total += x_component * i
                } else {
                    total += (x_component + z_component) / 2.
                }

                point + total.clamp(0., f32::INFINITY)
            };

        let mut a = points.into_iter().zip(mapping).map(select_corner);

        [
            a.next().unwrap(), // tl
            a.next().unwrap(), // tr
            a.next().unwrap(), // br
            a.next().unwrap(), // bl
        ]
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
/// Rotation bitflags are assumed to be clockwise
///
/// Flags:
/// * FlipX = 0b1;
/// * FlipY = 0b1 << 1;
/// * RotateClockwise = 0b1 << 2;
/// * RotateCounterClockwise = 0b1 << 3;
/// * FlipTriangles = 0b1 << 4;
/// * Exclusive = 0b1 << 5;
/// * Fold = 0b1 << 6;
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
