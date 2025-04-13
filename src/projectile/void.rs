use bevy::prelude::*;
#[derive(Debug, Component)]
pub struct VoidedProjectile;

impl super::ProjectileInterface for VoidedProjectile {
    fn build(&mut self, app: &mut App) {}

    fn on_spawn(
        &mut self,
        world: &mut World,
        projectile_entity: Entity,
        source: Option<Entity>,
        projectile_data: &super::interfaces::ProjectileData,
    ) {
    }
}
