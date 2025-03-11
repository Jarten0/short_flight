use crate::assets::{AssetStates, RonAssetLoader};
use anim_state::ShayminAnimation;
use assets::{AnimationAsset, ShayminAssets};
use bevy::prelude::*;
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use bevy_asset_loader::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use bevy_sprite3d::prelude::*;
use short_flight::animation::{self, AnimType};

mod anim_state;
mod assets;

/// Insert into world's that manage client-player state for the silly little goober :3
///
/// A Shaymin entity MUST exist at all times, it's effectively the manager for the client entity while these systems are active.
/// If the player "character" needs to be removed, make the logic conditional instead.
/// Hide it, end any systems, but do not remove it unless the world is being discarded anyways.
pub struct ShayminPlugin;

impl Plugin for ShayminPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn(Shaymin);
        app.add_systems(Startup, spawn_shaymin)
            .add_systems(OnEnter(AssetStates::Done), insert_assets)
            .add_systems(
                Update,
                (
                    control_shaymin,
                    process_shaymin_collisions,
                    anim_state::update_materials,
                )
                    .chain(),
            )
            .init_asset::<AnimationAsset>()
            .register_asset_loader::<RonAssetLoader<AnimationAsset>>(RonAssetLoader::default())
            .add_loading_state(
                LoadingState::new(AssetStates::PlayerLoading)
                    .continue_to_state(AssetStates::Done)
                    .load_collection::<ShayminAssets>()
                    .on_failure_continue_to_state(AssetStates::Retry),
            )
            .add_systems(OnEnter(AssetStates::Retry), retry);
    }
}

fn retry(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        AnimationAsset(hash_map)
    };
    short_flight::serialize_to_file(&asset, "assets/shaymin/animations.ron");
    let animations = asset_server.add(asset);
    commands.insert_resource(ShayminAssets {
        shaymin,
        animations,
    });
    commands.set_state(AssetStates::Done);
}

/// Marker component for the client state parent entity.
/// Used primarily for player logic.
#[derive(Debug, Component, Reflect, Clone)]
pub struct Shaymin;

pub type Client<'a> = Single<'a, Entity, With<Shaymin>>;
pub type ClientQuery<'a, T, F = ()> = Single<'a, T, (With<Shaymin>, F)>;

fn spawn_shaymin(shaymin: Client, mut commands: Commands, asset_server: Res<AssetServer>) {
    // let mesh = asset_server.add(Cuboid::from_size(Vec3::ONE / 2.0).into());
    // let material = asset_server.add::<StandardMaterial>(Color::srgb(0.3, 0.6, 0.25).into());
    commands.entity(*shaymin).insert((
        // Mesh3d(mesh),
        // MeshMaterial3d(material),
        Transform::from_xyz(10.0, 1.5, -2.0)
            .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0))),
    ));
}

fn insert_assets(
    shaymin: Client,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<ShayminAssets>,
    anim_assets: Res<Assets<AnimationAsset>>,
    sprite3d_params: Sprite3dParams,
) {
    commands.entity(*shaymin).insert((
        anim_state::animation(&asset_server, &assets, anim_assets),
        anim_state::sprite(&assets, sprite3d_params),
    ));
    log::info!("Inserted shaymin assets");
}

fn control_shaymin(
    shaymin: ClientQuery<(&mut Transform, Option<&mut ShayminAnimation>), Without<Camera3d>>,
    camera: Option<Single<&mut Transform, With<Camera3d>>>,
    kb: Res<ButtonInput<KeyCode>>,
    delta: Res<Time<Fixed>>,
) {
    let (mut transform, mut anim) = shaymin.into_inner();
    let mut cam_transform = camera.unwrap().into_inner();

    let Some(mut anim) = anim else {
        return;
    };

    let current = anim.current;
    let data = anim.pool.get_mut(&current).unwrap();

    if data.can_move() {
        if let Some(movement) = manage_movement(kb, &mut transform, &delta) {
            anim.direction = movement.xy();
            if current == animation::Idle {
                anim.current = animation::Walking;
            }
        };
    }
    cam_transform.translation = {
        let mut vec3 = transform.translation;
        vec3.y += 10.0;
        vec3
    };
}

fn manage_movement(
    kb: Res<ButtonInput<KeyCode>>,
    transform: &mut Mut<Transform>,
    delta: &Res<Time<Fixed>>,
) -> Option<Vec3> {
    if kb.pressed(KeyCode::ShiftLeft) {
        return None;
    }
    if kb.pressed(KeyCode::Space) {
        return None;
    }

    let input = {
        let mut dir: Vec3 = Vec3::ZERO;
        if kb.pressed(KeyCode::KeyA) {
            dir += Vec3::NEG_X
        }
        if kb.pressed(KeyCode::KeyD) {
            dir += Vec3::X
        }
        if kb.pressed(KeyCode::KeyW) {
            dir += Vec3::NEG_Z
        }
        if kb.pressed(KeyCode::KeyS) {
            dir += Vec3::Z
        }

        dir
    };
    let movement = input / 1.5 * delta.delta_secs();
    transform.translation += movement;
    return Some(movement);
}

fn process_shaymin_collisions(
    mut shaymin: Option<Single<(&mut Transform, &Shaymin), Without<Camera3d>>>,
    mut walls: Query<&TileStorage, Without<Camera3d>>,
) {
}
