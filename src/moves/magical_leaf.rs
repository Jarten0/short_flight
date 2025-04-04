use super::prelude::*;
#[derive(Component, Reflect)]
pub(super) struct MagicalLeaf;

impl MoveComponent for MagicalLeaf {
    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, process);
    }
}

fn process() {}
