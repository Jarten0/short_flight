use crate::assets::AnimationAssets;
use crate::assets::ShortFlightLoadingState;
use crate::npc::animation::AnimationHandler;
use assets::ShayminAssets;
use bevy::prelude::*;
use short_flight::animation::{AnimType, AnimationDirLabel};
use short_flight::sprite3d::Sprite3dParams;

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
            .add_systems(
                OnEnter(ShortFlightLoadingState::PlayerLoading),
                insert_animation,
            )
            .add_systems(OnEnter(ShortFlightLoadingState::Done), insert_sprite)
            .add_systems(FixedFirst, physics::update_dynamic_collision)
            .add_systems(
                FixedUpdate,
                (physics::control_shaymin, physics::update_rigidbodies).chain(),
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
fn insert_animation(
    shaymin: Client,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<ShayminAssets>,
) {
    let animation = anim_state::animation(&asset_server, &assets);
    commands
        .entity(*shaymin)
        .insert((animation, InheritedVisibility::VISIBLE));

    log::info!("Inserted shaymin assets");
}

fn insert_sprite(
    mut sprite_3d_params: Sprite3dParams,
    client: ClientQuery<(Entity, &AnimationHandler)>,
    mut commands: Commands,
    assets: Res<ShayminAssets>,
) {
    let sprite = anim_state::sprite(&assets).bundle_with_atlas(
        &mut sprite_3d_params,
        TextureAtlas {
            layout: client.1.spritesheet.atlas.clone().unwrap(),
            index: 0,
        },
    );
    commands.entity(client.0).with_child((
        Name::new("3D Sprite"),
        sprite,
        Transform::from_xyz(0.0, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0))),
        ClientChild,
    ));
}

#[derive(Debug, Component)]
pub struct ClientChild;

/// Runs if any of the assets cannot be loaded
fn retry(mut commands: Commands, asset_server: Res<AssetServer>) {
    log::info!("Could not load assets, initializing failsafe");
    let shaymin = asset_server.load::<Image>("shaymin/shaymin.png");
    let asset = {
        let hash_map = [
            AnimType::Idle.create_data(1, AnimationDirLabel::None),
            AnimType::Walking.create_data(1, AnimationDirLabel::None),
            AnimType::Hurt.create_data(1, AnimationDirLabel::None),
            AnimType::Down.create_data(1, AnimationDirLabel::None),
            AnimType::AttackSwipe.create_data(1, AnimationDirLabel::None),
            AnimType::AttackTackle.create_data(1, AnimationDirLabel::None),
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
