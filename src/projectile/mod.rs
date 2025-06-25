use bevy::prelude::*;
use bevy_asset_loader::mapped::MapKey;
use enum_iterator::Sequence;
pub use interfaces::ProjectileInterface;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Component,
    Default,
    Reflect,
    Sequence,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
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
    use super::{Projectile, register_interface};
    use crate::assets::AnimationSpritesheet;
    use crate::billboard::Billboard;
    use crate::collision::{
        BasicCollider, ColliderShape, CollisionLayers, DynamicCollision, ZHitbox,
    };
    use crate::npc::animation::AnimationHandler;
    use crate::npc::stats::{Damage, FacingDirection};
    use crate::sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dBundle, Sprite3dParams};
    use bevy::asset::LoadState;
    use bevy::platform::collections::HashMap;
    use bevy::prelude::*;
    use bevy_asset_loader::asset_collection::AssetCollection;
    use serde::{Deserialize, Serialize};

    #[derive(Resource, AssetCollection)]
    pub(crate) struct ProjectileCatalog {
        #[asset(path = "projectile_data", collection(typed, mapped))]
        pub data_files: HashMap<Projectile, Handle<ProjectileData>>,

        #[asset(path = "projectiles", collection(typed, mapped))]
        pub image_files: HashMap<Projectile, Handle<Image>>,
    }

    /// Container for all information pertaining to a projectile type.
    ///
    ///
    #[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
    pub(crate) struct ProjectileData {
        pub(crate) variant: Projectile,
        pub(crate) display_name: String,
        pub(crate) spritesheet: AnimationSpritesheet,
        pub(crate) collider: ColliderShape,
        #[serde(default)]
        pub(crate) damage: Damage,
        #[serde(skip)]
        pub(crate) assets: Option<ProjectileAssets>,
    }

    /// Container for handles that a projectile will use.
    /// Initialized once per projectile type.
    #[derive(Debug, Reflect, Clone)]
    pub(crate) struct ProjectileAssets {
        pub mesh: Handle<Mesh>,
        pub texture: Handle<Image>,
        pub material: Handle<StandardMaterial>,
        pub data: Handle<ProjectileData>,
        pub atlas: Handle<TextureAtlasLayout>,
        pub sprite3d_base: Sprite3d,
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
            source: Option<Entity>,
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
            for (_, registration) in interfaces.iter_mut() {
                registration.build(app);
            }
            app.insert_resource(interfaces)
                .add_systems(Update, validate_projectile_data);
        }
    }

    #[derive(Resource, Deref, DerefMut)]
    pub struct ProjectileInterfaces(HashMap<Projectile, Box<dyn ProjectileInterface>>);

    pub struct SpawnProjectile {
        pub source: Option<Entity>,
        pub position: Vec3,
        pub direction: Dir2,
        pub projectile_id: Projectile,
    }

    impl Command for SpawnProjectile {
        fn apply(self, world: &mut World) {
            let display = match self
                .source
                .map(|entity| world.get::<Name>(entity))
                .unwrap_or_default()
            {
                Some(name) => name.as_str().to_string(),
                None => format!("{:?}", self.source),
            };

            let catalog = world.resource::<ProjectileCatalog>();
            let data_assets = world.resource::<Assets<ProjectileData>>();
            let image = catalog
                .image_files
                .get(&self.projectile_id)
                .expect("The projectile catalog is missing an image variant.")
                .clone_weak();

            let handle = catalog.data_files.get(&self.projectile_id).expect(
                "The projectile catalog MUST exhaustively contain all projectile variants.",
            );
            let data = data_assets.get(handle).expect(
                "SpawnProjectile should not be called before all projectile data assets are loaded.",
            ).clone();

            let assets = data.assets.as_ref().unwrap();

            let asset_server = world.resource::<AssetServer>();

            #[cfg(debug_assertions)]
            for (handle, item) in [
                (assets.atlas.clone_weak().untyped(), "atlas"),
                (assets.data.clone_weak().untyped(), "data"),
                (assets.texture.clone_weak().untyped(), "texture"),
                (assets.mesh.clone_weak().untyped(), "mesh"),
                (assets.material.clone_weak().untyped(), "material"),
            ] {
                debug_assert!(
                    asset_server.is_loaded(&handle),
                    "{} projectile {} asset is not loaded! [{:?}], Load state: [{:?}]",
                    display,
                    item,
                    handle,
                    asset_server.get_load_state(&handle)
                )
            }

            let id = world
                .spawn((
                    self.projectile_id,
                    FacingDirection(self.direction),
                    AnimationHandler::new(data.spritesheet.clone()),
                    BasicCollider::new(
                        true,
                        data.collider.clone(),
                        CollisionLayers::Projectile & CollisionLayers::Attack,
                        CollisionLayers::NPC,
                    ),
                    ZHitbox {
                        y_tolerance: 0.5,
                        neg_y_tolerance: 0.0,
                    },
                    Transform::from_translation(self.position)
                        .with_rotation(Quat::from_rotation_x(f32::to_radians(-90.0))),
                    data.damage.clone(),
                    DynamicCollision::default(),
                    Sprite3dBundle {
                        sprite_3d: assets.sprite3d_base.clone(),
                        mesh: bevy::prelude::Mesh3d(assets.mesh.clone_weak()),
                        material: bevy::prelude::MeshMaterial3d(assets.material.clone_weak()),
                    },
                    Billboard::default(),
                ))
                .id();

            log::info!(
                "Spawning projectile: Source [{}] ID [{:?}] Entity [{}]",
                display,
                self.projectile_id,
                id
            );

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

    pub(crate) fn validate_projectile_data(
        mut asset_events: EventReader<AssetEvent<ProjectileData>>,
        mut projectile_data: ResMut<Assets<ProjectileData>>,
        catalog: Option<Res<ProjectileCatalog>>,
        asset_server: Res<AssetServer>,
        mut sprite3d_params: Sprite3dParams,
    ) {
        let Some(catalog) = catalog else { return };
        for event in asset_events.read() {
            match event {
                AssetEvent::Added { id } => {
                    let data_handle = projectile_data.get_strong_handle(*id).unwrap();
                    let data = projectile_data.get_mut(*id).unwrap();
                    log::info!("Validating projectile: {:?}", data.variant);
                    data.spritesheet.atlas =
                        Some(asset_server.add(data.spritesheet.get_atlas_layout()));

                    let Sprite3dBundle {
                        sprite_3d: sprite3d_base,
                        mesh,
                        material,
                    } = Sprite3dBuilder {
                        image: catalog
                            .image_files
                            .get(&data.variant)
                            .expect("Image file not found for variant")
                            .clone(),
                        pixels_per_metre: 32.0,
                        pivot: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        unlit: true,
                        double_sided: true,
                        emissive: LinearRgba::BLACK,
                    }
                    .bundle(&mut sprite3d_params, &asset_server);

                    data.assets = Some(ProjectileAssets {
                        mesh: mesh.0,
                        material: material.0,
                        sprite3d_base,
                        texture: catalog
                            .image_files
                            .get(&data.variant)
                            .expect("Image file not found for variant")
                            .clone(),
                        data: data_handle,
                        atlas: data.spritesheet.atlas.as_ref().unwrap().clone(),
                    });

                    log::info!("Validated")
                }
                _ => (),
            }
        }
    }
}
