use super::stats::Damage;
use super::stats::Health;
use super::NPCInfo;
use super::NPC;
use crate::assets::AnimationSpritesheet;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_asset_loader::asset_collection::AssetCollection;
use serde::Deserialize;
use serde::Serialize;
use short_flight::collision::Collider;

#[derive(Resource, AssetCollection)]
pub(crate) struct NPCAlmanac {
    #[asset(path = "npc_data", collection(typed, mapped))]
    pub data_files: HashMap<NPC, Handle<NPCData>>,

    #[asset(path = "npcs", collection(typed, mapped))]
    pub image_files: HashMap<NPC, Handle<Image>>,
}

#[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
pub(crate) struct NPCData {
    pub(crate) display_name: String,
    pub(crate) info: NPCInfo,
    pub(crate) collider: Option<Collider>,
    pub(crate) stats: Option<(Health, Damage)>,
    pub(crate) spritesheet: AnimationSpritesheet,
}

pub(crate) fn validate_npc_data(
    mut npc_datas: ResMut<Assets<NPCData>>,
    asset_server: Res<AssetServer>,
) {
    for (id, data) in npc_datas.iter_mut() {
        assert!(data.stats.is_some());
        assert!(data.collider.is_some());
        data.spritesheet.atlas = Some(asset_server.add(data.spritesheet.get_texture_atlas()));
        assert!(data.spritesheet.atlas.is_some());
    }
}

// pub(crate) fn load_npcs(asset_server: Res<AssetServer>, mut commands: Commands) {
//     let handles = enum_iterator::all().map(|npc: NPC| {
//         let path = "npcs/".to_string() + npc.variant_name();
//         let npc_data = asset_server.load::<NPCData>(path.clone() + ".ron");

//         let npc_texture = asset_server.load::<Image>(path.clone() + ".png");
//         (npc, (npc_data, npc_texture))
//     });

//     commands.insert_resource(NPCAlmanac {
//         data_files: handles.clone().map(|(id, asset)| ),
//         image_files: HashMap::new(),
//     });
// }

// pub(crate) fn validate_almanac(
//     almanac: Res<NPCAlmanac>,
//     npc_data: ResMut<Assets<NPCData>>,
//     asset_server: Res<AssetServer>,
// ) {
//     for (npc, data) in almanac.data_files.iter() {
//         match asset_server.load_state(data) {
//             LoadState::NotLoaded => unimplemented!(),
//             LoadState::Loading => (),
//             LoadState::Loaded => (),
//             LoadState::Failed(asset_load_error) => {
//                 let path = "assets/".to_string() + &data.path().unwrap().path().to_string_lossy();
//                 if std::fs::exists(&path).unwrap() {
//                     panic!("Asset failed to load {}", asset_load_error);
//                 }
//                 log::info!("Creating default asset at: {}", &path);
//                 File::create(path)
//                     .unwrap()
//                     .write_all(
//                         ron::ser::to_string_pretty(
//                             &NPCData {
//                                 display_name: String::from("Void"),
//                                 info: NPCInfo::Enemy {},
//                                 collider: Some(Collider {
//                                     dynamic: false,
//                                     shape: short_flight::collision::ColliderShape::Circle {
//                                         radius: 1.0,
//                                     },
//                                     layers: CollisionLayers::default(),
//                                     can_interact: CollisionLayers::default(),
//                                 }),
//                                 stats: Some((Health::new(1), Damage(1))),
//                                 spritesheet: AnimationSpritesheet {
//                                     animations: Vec::new(),
//                                     sprite_size: UVec2::ZERO,
//                                     data: AnimationAssets(std::collections::HashMap::from_iter([
//                                         (
//                                             AnimType::Idle,
//                                             AnimationData {
//                                                 variant: AnimType::Idle,
//                                                 frames: 2,
//                                                 can_move_override: None,
//                                             },
//                                         ),
//                                     ])),
//                                     atlas: None,
//                                     texture: None,
//                                 },
//                             },
//                             PrettyConfig::new().struct_names(true),
//                         )
//                         .unwrap()
//                         .as_bytes(),
//                     )
//                     .unwrap();
//                 panic!("Retry now that there should be a good asset");
//             }
//         }
//     }
//     log::info!("Validated NPC data for {} npcs", almanac.data_files.len());
// }
