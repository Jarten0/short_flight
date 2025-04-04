use std::collections::HashMap;

use bevy::asset::Asset;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
pub use AnimType::*;

/// lightweight identifier for variants of animations
/// a seperate struct is used to store all of the common animation data
#[derive(
    Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Reflect, Serialize, Deserialize,
)]
pub enum AnimType {
    #[default]
    Idle,
    Walking,
    AttackSwipe,
    AttackTackle,
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
    fn use_timer(self) -> bool {
        match self {
            Idle => false,
            _ => true,
        }
    }
    fn default_animation_time(self) -> f32 {
        match self {
            Idle => 0.0,
            Walking => 0.0,
            AttackSwipe => 2.0,
            AttackTackle => 2.0,
            _ => 1.0,
        }
    }
    pub fn create_data(self) -> AnimationData {
        AnimationData {
            variant: self,
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
    #[serde(flatten)]
    #[serde(default)]
    #[serde(alias = "can_move")]
    /// set a specific value for can_move for this animation
    blocked_override: Option<bool>,
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
            return self.variant.use_timer();
        }

        return false;
    }
    /// returns true if this animation allows the player to move
    pub fn is_blocking(&self) -> bool {
        if let Some(ovr) = self.blocked_override {
            return ovr;
        }
        return self.variant.blocks();
    }
}
