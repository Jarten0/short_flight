#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(path_add_extension)]

use bevy::prelude::*;

pub(crate) mod animation;
pub(crate) mod camera;
pub(crate) mod collision;
pub(crate) mod editor;
pub(crate) mod sprite3d;

mod assets;
mod ldtk;
mod mesh;
mod moves;
mod npc;
mod player;
mod projectile;
mod tile;

fn main() {
    App::new()
        // builtin
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(MeshPickingPlugin)
        // game
        .add_plugins(assets::AssetsPlugin)
        .add_plugins(npc::NPCPlugin)
        .add_plugins(moves::interfaces::MovePlugin)
        .add_plugins(projectile::interfaces::ProjectilePlugin)
        .add_plugins(player::ShayminPlugin)
        .add_plugins(ldtk::LdtkPlugin)
        // lib
        .add_plugins(camera::CustomCameraPlugin)
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(mesh::TileMeshManagerPlugin)
        .add_plugins(sprite3d::Sprite3dPlugin)
        // third party
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        .add_plugins(bevy_editor_cam::DefaultEditorCamPlugins)
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(bevy::remote::http::RemoteHttpPlugin::default())
        .run();
}
