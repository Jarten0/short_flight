use bevy::prelude::*;
use bevy_asset_loader::mapped::MapKey;
use enum_iterator::Sequence;
pub use interfaces::ProjectileInterface;
use serde::{Deserialize, Serialize};

#[derive(
    Component, Default, Reflect, Sequence, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Hash,
)]
pub enum Projectile {
    #[default]
    Void = 0,
    LeafAttack,
}

mod leaf_attack;
mod void;

fn register_interface(input: Projectile) -> Box<dyn ProjectileInterface> {
    match input {
        Projectile::Void => Box::new(void::VoidedProjectile),
        Projectile::LeafAttack => Box::new(leaf_attack::LeafAttack),
    }
}

impl MapKey for Projectile {
    fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
        short_flight::from_asset_path(path)
    }
}

pub mod interfaces {
    use super::{register_interface, Projectile};
    use crate::assets::AnimationSpritesheet;
    use crate::npc::animation::AnimationHandler;
    use bevy::ecs::system::SystemState;
    use bevy::prelude::*;
    use bevy::utils::hashbrown::HashMap;
    use bevy_asset_loader::asset_collection::AssetCollection;
    use serde::{Deserialize, Serialize};
    use short_flight::collision::{BasicCollider, ColliderShape, CollisionLayers, ZHitbox};
    use short_flight::sprite3d::{Sprite3dBuilder, Sprite3dParams};

    #[derive(Resource, AssetCollection)]
    pub(crate) struct ProjectileCatalog {
        #[asset(path = "projectile_data", collection(typed, mapped))]
        pub data_files: HashMap<Projectile, Handle<ProjectileData>>,

        #[asset(path = "projectiles", collection(typed, mapped))]
        pub image_files: HashMap<Projectile, Handle<Image>>,
    }

    #[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
    pub(crate) struct ProjectileData {
        pub(crate) display_name: String,
        pub(crate) spritesheet: AnimationSpritesheet,
        pub(crate) collider: ColliderShape,
    }

    pub trait ProjectileInterface: Send + Sync {
        /// Initialize any useful schedules here.
        ///
        /// Is called for every variant with an associated data file.
        fn build(&mut self, app: &mut App);

        fn on_spawn(
            &mut self,
            world: &mut World,
            projectile_entity: Entity,
            source: Entity,
            projectile_data: &ProjectileData,
        ) {
            // world.entity_mut(projectile_entity).insert(Self);
        }
    }

    pub struct ProjectilePlugin;

    impl Plugin for ProjectilePlugin {
        fn build(&self, app: &mut App) {
            let mut interfaces = ProjectileInterfaces(
                enum_iterator::all::<Projectile>()
                    .into_iter()
                    .map(|input| (input, register_interface(input)))
                    .collect(),
            );
            interfaces.iter_mut().for_each(|(_, registration)| {
                registration.build(app);
            });
            app.insert_resource(interfaces);
            app.add_systems(PreUpdate, validate_npc_data);
        }
    }

    #[derive(Resource, Deref, DerefMut)]
    pub struct ProjectileInterfaces(HashMap<Projectile, Box<dyn ProjectileInterface>>);

    pub struct SpawnProjectile {
        pub source: Entity,
        pub projectile_id: Projectile,
    }

    impl Command for SpawnProjectile {
        fn apply(self, world: &mut World) {
            let catalog = world.resource::<ProjectileCatalog>();
            let data_assets = world.resource::<Assets<ProjectileData>>();
            let image_handle = catalog
                .image_files
                .get(&self.projectile_id)
                .expect("The projectile catalog MUST exhaustively contain all projectile variants.")
                .clone_weak();
            let handle = catalog.data_files.get(&self.projectile_id).expect(
                "The projectile catalog MUST exhaustively contain all projectile variants.",
            );
            let data = data_assets.get(handle).expect(
                "SpawnProjectile should not be called before all projectile data assets are loaded.",
            ).clone();

            let id = world
                .spawn((
                    self.projectile_id,
                    AnimationHandler::new(data.spritesheet.clone()),
                    BasicCollider::new(
                        true,
                        data.collider.clone(),
                        CollisionLayers::Projectile,
                        CollisionLayers::NPC,
                    ),
                    ZHitbox {
                        y_tolerance: 0.5,
                        neg_y_tolerance: 0.0,
                    },
                ))
                .id();

            let mut system_state: SystemState<Sprite3dParams> = SystemState::new(world);

            let sprite_3d_bundle = Sprite3dBuilder {
                image: image_handle,
                pixels_per_metre: 32.0,
                pivot: None,
                alpha_mode: AlphaMode::Mask(0.5),
                unlit: false,
                double_sided: true,
                emissive: LinearRgba::BLACK,
            }
            .bundle_with_atlas(
                &mut system_state.get_mut(world),
                TextureAtlas {
                    layout: data.spritesheet.atlas.clone().unwrap(),
                    index: 0,
                },
            );

            world.entity_mut(id).insert(sprite_3d_bundle);

            system_state.apply(world);

            world.resource_scope(
                |world, mut projectile_interfaces: Mut<ProjectileInterfaces>| {
                    projectile_interfaces
                        .get_mut(&self.projectile_id)
                        .expect(
                            "Expected ProjectileInterfaces to exhaustively contain every variant",
                        )
                        .on_spawn(world, id, self.source, &data);
                },
            );
        }
    }

    pub(crate) fn validate_npc_data(
        mut asset_events: EventReader<AssetEvent<ProjectileData>>,
        mut npc_datas: ResMut<Assets<ProjectileData>>,
        asset_server: Res<AssetServer>,
    ) {
        for event in asset_events.read() {
            match event {
                AssetEvent::Added { id } => {
                    let data = npc_datas.get_mut(*id).unwrap();
                    data.spritesheet.atlas =
                        Some(asset_server.add(data.spritesheet.get_atlas_layout()));
                }
                _ => (),
            }
        }

        for (id, data) in npc_datas.iter_mut() {
            assert!(data.spritesheet.atlas.is_some());
        }
    }
}
