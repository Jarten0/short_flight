use crate::npc::animation::NPCAnimation;

use super::prelude::*;
#[derive(Component, Reflect)]
pub(super) struct Tackle;

impl MoveComponent for Tackle {
    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, tackle);
    }
}
fn tackle(
    active_moves: Query<(&Tackle, &Parent)>,
    mut parent: Query<(&mut Transform, &NPCAnimation)>,
    time: Res<Time>,
) {
    for (tackle, entity) in active_moves.iter() {
        let (mut transform, anim) = parent.get_mut(**entity).unwrap();

        transform.translation += anim.direction * time.delta_secs() * 2.;
    }
}
