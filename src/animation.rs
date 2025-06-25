pub use AnimType::*;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::npc::stats::FacingDirection;

/// lightweight identifier for variants of animations
/// a seperate struct is used to store all of the common animation data
#[derive(
    Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Reflect, Serialize, Deserialize,
)]
pub enum AnimType {
    /// default, with direction as right
    #[default]
    Idle,
    Walking,
    AttackSwipe,
    AttackTackle,
    AttackShoot,
    Hurt,
    Down,
    /// The NPC has lost all HP and is playing faint animation
    Fainting,
    /// Plays if NPC does not despawn on faint.
    Fainted,
}

// add default properties to variants here
impl AnimType {
    fn can_interrupt(self, other: Option<Self>) -> bool {
        match self {
            Idle => true,
            Walking => true,
            AttackSwipe => match other.unwrap_or_default() {
                Hurt => true,
                _ => false,
            },
            AttackTackle => match other.unwrap_or_default() {
                Hurt => true,
                _ => false,
            },
            AttackShoot => match other.unwrap_or_default() {
                Hurt => true,
                _ => false,
            },
            _ => false,
        }
    }
    /// If `true`, then the NPC cannot perform different actions until this animation is over
    fn blocks(self) -> bool {
        match self {
            Idle => false,
            Walking => false,
            _ => true,
        }
    }
    fn reset_timer(self) -> bool {
        match self {
            Idle => false,
            _ => true,
        }
    }
    pub fn create_data(self, frames: u32, directions: AnimationDirLabel) -> AnimationData {
        AnimationData {
            variant: self,
            frames,
            direction_label: directions,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Reflect, Clone, Serialize, Deserialize)]
pub struct AnimationData {
    #[serde(alias = "type")]
    pub variant: AnimType,
    #[serde(default)]
    #[serde(alias = "length")]
    #[serde(alias = "time")]
    pub frames: u32,
    #[serde(default)]
    #[serde(alias = "direction")]
    pub direction_label: AnimationDirLabel,
    #[serde(flatten)]
    #[serde(default)]
    #[serde(alias = "can_move")]
    /// set a specific value for can_move for this animation
    pub blocking_override: Option<bool>,
}

impl AnimationData {
    // pub const fn time(mut self, time: f32) -> Self {
    //     self.time = time;
    //     self
    // }
    // pub const fn can_move_override(mut self, can_move: bool) -> Self {
    //     self.can_move_override = Some(can_move);
    //     self
    // }
    /// returns true when the animation is over and should be switched to idle
    pub fn process_timer(&self, frame: &mut f32, delta: f32) -> bool {
        *frame += delta;

        if *frame >= self.frames as f32 {
            *frame = 0.0;
            return self.variant.reset_timer();
        }

        return false;
    }
    /// returns true if this animation allows the player to move
    pub fn is_blocking(&self) -> bool {
        if let Some(ovr) = self.blocking_override {
            return ovr;
        }
        return self.variant.blocks();
    }
}

/// Label for what direction(s) this animation is facing.
/// Depending on the animation, it may have multiple directions.
///
/// By default, horizontal variants should face [`Dir2::EAST`] and vertical (or non-directional) variants should face [`Dir2::SOUTH`].
///
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default, Reflect)]
pub enum AnimationDirLabel {
    #[default]
    /// This animation always appears like this regardless of orientation.
    None,
    /// Has a secondary, vertical variant.
    Vertical,
    /// Has a secondary, horizontal variant.
    Horizontal,
    /// Has both a horizontal and vertical variant, which can be flipped to get opposite directions.
    FlipVariants,
    /// Has a "front" and "back" variant, as well as a "Side" horizontal variant that is flipped for side views.
    FrontBackAndHorizontal,
    /// No need to flip, all four directions are contained
    FullyDirectional,
}

impl AnimationDirLabel {
    pub fn directional_sprite_count(self) -> u32 {
        match self {
            AnimationDirLabel::None => 1,
            AnimationDirLabel::Vertical => 2,
            AnimationDirLabel::Horizontal => 2,
            AnimationDirLabel::FlipVariants => 2,
            AnimationDirLabel::FrontBackAndHorizontal => 3,
            AnimationDirLabel::FullyDirectional => 4,
        }
    }

    /// 0. index
    /// 1. flip
    pub fn get_index_offset(self, dir: &FacingDirection) -> (usize, BVec2) {
        let cardinal_dir = cardinal(**dir);
        let horizontal_dir =
            Dir2::new(dir.with_y(0.).try_normalize().unwrap_or(Vec2::X)).unwrap_or(Dir2::EAST);
        let vertical_dir =
            Dir2::new(dir.with_x(0.).try_normalize().unwrap_or(Vec2::NEG_Y)).unwrap_or(Dir2::SOUTH);

        match self {
            AnimationDirLabel::None => (0, BVec2::FALSE),
            AnimationDirLabel::Horizontal => match horizontal_dir {
                Dir2::EAST => (0, BVec2::FALSE),
                Dir2::WEST => (0, BVec2 { x: true, y: false }),
                _ => panic!("Impossible(?) direction variant matched."),
            },
            AnimationDirLabel::Vertical => match vertical_dir {
                Dir2::SOUTH => (0, BVec2::FALSE),
                Dir2::NORTH => (0, BVec2 { x: false, y: true }),
                _ => panic!("Impossible(?) direction variant matched."),
            },
            AnimationDirLabel::FlipVariants => match cardinal_dir {
                Dir2::NORTH => (0, BVec2::FALSE),
                Dir2::SOUTH => (0, BVec2 { x: false, y: true }),
                Dir2::EAST => (1, BVec2::FALSE),
                Dir2::WEST => (1, BVec2 { x: true, y: false }),
                _ => panic!("Impossible(?) direction variant matched."),
            },
            AnimationDirLabel::FrontBackAndHorizontal => match cardinal_dir {
                Dir2::NORTH => (0, BVec2::FALSE),
                Dir2::SOUTH => (1, BVec2::FALSE),
                Dir2::EAST => (2, BVec2::FALSE),
                Dir2::WEST => (2, BVec2 { x: true, y: false }),
                _ => panic!("Impossible(?) direction variant matched."),
            },
            AnimationDirLabel::FullyDirectional => match cardinal_dir {
                Dir2::EAST => (0, BVec2::FALSE),
                Dir2::NORTH => (1, BVec2::FALSE),
                Dir2::WEST => (2, BVec2::FALSE),
                Dir2::SOUTH => (3, BVec2::FALSE),
                _ => panic!("Impossible(?) direction*alt variant matched."),
            },
        }
    }
}

pub fn cardinal(input: Dir2) -> Dir2 {
    Dir2::new(if input.x.abs() >= input.y.abs() {
        input.with_y(0.0).normalize()
    } else if input.y.abs() >= input.x.abs() {
        input.with_x(0.0).normalize()
    } else {
        input.with_x(0.0).normalize()
    })
    .expect("bath mamphs ;(")
}
