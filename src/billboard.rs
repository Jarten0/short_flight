use std::sync::OnceLock;

use bevy::prelude::*;

/// Marker for single entity which determines which way targetted billboard entities should face.
#[derive(Debug, Component)]
#[relationship_target(relationship = Billboard)]
#[require(Transform)]
pub struct BillboardTarget(Vec<Entity>);

/// Marker for entities to be consistently rotated towards a [`BillboardTarget`].
#[derive(Debug, Component)]
#[relationship(relationship_target = BillboardTarget)]
#[require(Transform)]
pub struct Billboard {
    target: Entity,
}

pub(crate) static DEFAULT_BILLBOARD_TARGET: OnceLock<Entity> = OnceLock::new();

impl Default for Billboard {
    /// Uses the default billboard target (the camera) to avoid unnecessary and repetitive access through bevy's ECS.
    fn default() -> Self {
        Self {
            target: *DEFAULT_BILLBOARD_TARGET
                .get()
                .expect("Default billboard target not initialized!"),
        }
    }
}

pub(crate) fn update_billboards(
    target: Single<&GlobalTransform, With<BillboardTarget>>,
    billboards: Query<(&mut Transform, &GlobalTransform), With<Billboard>>,
) {
    for (mut transform, gtransform) in billboards {
        let axis = (target.translation() - gtransform.translation()).normalize_or(Vec3::Y);
        let base_quat = Quat::from_rotation_x(f32::to_radians(-90.0));
        transform.rotation = Quat::from_axis_angle(axis, 0.0_f32.to_radians()) * base_quat;
    }
}
