use crate::assets::AnimationAssets;
use crate::assets::{RonAssetLoader, ShortFlightLoadingState};
use assets::ShayminAssets;
use bevy::prelude::*;
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use bevy_asset_loader::prelude::*;
use bevy_sprite3d::prelude::*;
use short_flight::animation::AnimType;

mod anim_state;
pub mod assets;
mod physics;

/// Insert into world's that manage client-player state for the silly little goober :3
///
/// A Shaymin entity MUST exist at all times, it's effectively the manager for the client entity while these systems are active.
/// If the player "character" needs to be removed, make the logic conditional instead.
/// Hide it, end any systems, but do not remove the entity nor the marker component unless the world is already being dropped anyways.
pub struct ShayminPlugin;

impl Plugin for ShayminPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn(Shaymin);
        app.add_systems(Startup, (setup, physics::setup))
            .add_systems(OnEnter(ShortFlightLoadingState::Done), insert_assets)
            .add_systems(FixedFirst, physics::update_rigidbodies)
            .add_systems(
                FixedUpdate,
                (physics::control_shaymin, anim_state::update_materials).chain(),
            )
            .add_systems(PostUpdate, (physics::draw_colliders).chain())
            .add_systems(OnEnter(ShortFlightLoadingState::FailState), retry);
    }
}

/// Marker component for the client state parent entity.
/// Used primarily for player logic.
#[derive(Debug, Component, Reflect, Clone)]
pub struct Shaymin;

pub type Client<'a> = Single<'a, Entity, With<Shaymin>>;
pub type ClientQuery<'a, T, F = ()> = Single<'a, T, (With<Shaymin>, F)>;

fn setup(shaymin: Client, mut commands: Commands) {
    commands
        .entity(*shaymin)
        .insert((Transform::from_xyz(10.0, 0.0, -2.0)));
}

/// Runs after all of the assets are loaded
fn insert_assets(
    shaymin: Client,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<ShayminAssets>,
    anim_assets: Res<Assets<AnimationAssets>>,
    sprite3d_params: Sprite3dParams,
) {
    commands
        .entity(*shaymin)
        .insert((
            anim_state::animation(&asset_server, &assets, anim_assets),
            InheritedVisibility::VISIBLE,
        ))
        .with_child((
            Name::new("3D Sprite"),
            anim_state::sprite(&assets, sprite3d_params),
            Transform::from_xyz(0.0, 1.0, 0.0)
                .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0))),
        ));
    log::info!("Inserted shaymin assets");
}

/// Runs if any of the assets cannot be loaded
fn retry(mut commands: Commands, asset_server: Res<AssetServer>) {
    log::info!("Could not load assets, initializing failsafe");
    let shaymin = asset_server.load::<Image>("shaymin/shaymin.png");
    let asset = {
        let hash_map = [
            AnimType::Idle.new(),
            AnimType::Walking.new(),
            AnimType::Hurt.new(),
            AnimType::Down.new(),
            AnimType::AttackSwipe.new(),
            AnimType::AttackTackle.new(),
        ]
        .into_iter()
        .map(|animation| (animation.variant, animation))
        .collect();
        AnimationAssets(hash_map)
    };
    let animations = asset_server.add(asset);
    commands.insert_resource(ShayminAssets {
        shaymin,
        animations,
    });
    // commands.set_state(ShortFlightLoadingState::Done);
}
