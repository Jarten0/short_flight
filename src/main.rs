#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(path_add_extension)]
#![feature(let_chains)]
#![feature(slice_as_array)]

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;

pub(crate) mod animation;
pub(crate) mod camera;
pub(crate) mod collision;
pub(crate) mod editor;
pub(crate) mod sprite3d;
pub(crate) mod billboard;

mod assets;
mod ldtk;
mod mesh;
mod moves;
mod npc;
mod projectile;
mod shaymin;
mod tile;

fn main() {
    App::new()
        // builtin
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    mode: AssetMode::Unprocessed,
                    file_path: "assets".to_string(),
                    processed_file_path: "imported_assets/Default".to_string(),
                    watch_for_changes_override: None,
                    meta_check: bevy::asset::AssetMetaCheck::default(),
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::default(),
                }),
        )
        .add_plugins(MeshPickingPlugin)
        // game
        .add_plugins(assets::AssetsPlugin)
        .add_plugins(npc::NPCPlugin)
        .add_plugins(moves::interfaces::MovePlugin)
        .add_plugins(projectile::interfaces::ProjectilePlugin)
        .add_plugins(shaymin::ShayminPlugin)
        .add_plugins(ldtk::LdtkPlugin)
        // lib
        .add_plugins(camera::CustomCameraPlugin)
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(mesh::TileMeshManagerPlugin)
        .add_plugins(sprite3d::Sprite3dPlugin)
        // third party
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                .run_if(|kb: Res<ButtonInput<KeyCode>>| kb.pressed(KeyCode::Backquote)),
        )
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        // .add_plugins(bevy_editor_cam::DefaultEditorCamPlugins)
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(bevy::remote::http::RemoteHttpPlugin::default())
        .add_systems(
            PreUpdate,
            |kb: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>| {
                if kb.just_pressed(KeyCode::Escape) && !kb.pressed(KeyCode::KeyC) {
                    log::info!("Escape key pressed, exiting...");
                }
                if kb.just_released(KeyCode::Escape) && !kb.pressed(KeyCode::KeyC) {
                    exit.write(AppExit::Success);
                }
            },
        )
        .run();
}

/// The conversion rate from LDTK pixels to ingame world units
pub const LDTK_PX_TO_WORLD: u32 = 32;

/// Conversion from pixel to world units
pub fn to_minsteps(input: f32) -> f32 {
    input / LDTK_PX_TO_WORLD as f32
}

pub fn to_minsteps_u32(input: u32) -> u32 {
    input / LDTK_PX_TO_WORLD
}

pub fn to_minsteps_i32(input: i32) -> i32 {
    input / LDTK_PX_TO_WORLD as i32
}
