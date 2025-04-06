use super::prelude::*;
#[derive(Component, Reflect)]
pub(super) struct MagicalLeaf;

impl MoveComponent for MagicalLeaf {
    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, process);
    }

    fn variant() -> super::Move
    where
        Self: Sized,
    {
        super::Move::MagicalLeaf
    }
}

fn process() {}
