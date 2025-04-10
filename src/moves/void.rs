use super::prelude::MoveComponent;
use bevy::prelude::*;

#[derive(Debug)]
pub struct VoidedMove;

impl MoveComponent for VoidedMove {
    fn build(&mut self, app: &mut App)
    where
        Self: Sized,
    {
    }
}
