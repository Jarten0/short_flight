use bevy::prelude::*;
use enum_iterator::Sequence;
use prelude::MoveComponent;
use serde::{Deserialize, Serialize};

mod prelude {
    pub use super::interfaces::MoveComponent;
    pub use bevy::prelude::*;
}

pub mod magical_leaf;
pub mod tackle;
pub mod void;

/// Marks this entity as a move, aka an attack, that temporarily exists in the world.
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
pub enum Move {
    #[default]
    Void = 0,
    Tackle,
    MagicalLeaf,
}

fn register_component(input: Move) -> Box<dyn MoveComponent> {
    match input {
        Move::Void => Box::new(void::VoidedMove),
        Move::Tackle => Box::new(tackle::Tackle),
        Move::MagicalLeaf => Box::new(magical_leaf::MagicalLeaf),
        // _ => void,
    }
}

pub mod interfaces {
    use super::Move;
    use super::prelude::*;
    use super::register_component;
    use crate::animation::AnimType;
    use crate::assets::AnimationSpritesheet;
    use crate::collision::ColliderShape;
    use crate::npc::animation::AnimationHandler;
    use crate::npc::stats::Damage;
    use bevy::platform::collections::HashMap;
    use bevy_asset_loader::asset_collection::AssetCollection;
    use bevy_asset_loader::mapped::MapKey;
    use serde::{Deserialize, Serialize};

    #[derive(
        Debug, Component, Reflect, Clone, Deref, DerefMut, Default, Serialize, Deserialize,
    )]
    #[serde(transparent)]
    pub struct Moves(pub Vec<Move>);

    /// This component operates with the move system
    pub trait MoveComponent: Send + Sync {
        /// Initialize any useful schedules here.
        ///
        /// Is called for every variant with an associated data file.
        fn build(&mut self, app: &mut App);

        /// Called whenever a move entity is spawned, right before this is inserted into the entity.
        ///
        /// `move_entity` is the entity that is automatically spawned for the move.
        fn on_spawn(&mut self, world: &mut World, move_entity: Entity, move_data: &MoveData) {
            // world.entity_mut(entity).insert(Self);
            // <Self as MoveComponent>::set_animation(world, entity, AnimType::AttackTackle, None);
        }

        fn set_animation(world: &mut World, move_entity: Entity, animation: AnimType)
        where
            Self: Sized,
        {
            world
                .get_mut::<AnimationHandler>(Self::parent(world, move_entity))
                .unwrap()
                .start_animation(animation)
        }

        fn parent(world: &World, move_entity: Entity) -> Entity
        where
            Self: Sized,
        {
            world.get::<ChildOf>(move_entity).unwrap().parent()
        }
    }

    pub struct MovePlugin;

    impl Plugin for MovePlugin {
        fn build(&self, app: &mut App) {
            let mut move_interfaces = MoveInterfaces(
                enum_iterator::all::<Move>()
                    .into_iter()
                    .map(|input| (input, register_component(input)))
                    .collect(),
            );
            move_interfaces.iter_mut().for_each(|(_, registration)| {
                registration.build(app);
            });
            app.insert_resource(move_interfaces);
        }
    }

    #[derive(Debug, Component)]
    pub struct MoveInfo {
        pub(crate) id: Move,
        pub(crate) data: Handle<MoveData>,
    }

    #[derive(Resource, AssetCollection)]
    pub(crate) struct MoveList {
        #[asset(path = "move_data", collection(typed, mapped))]
        pub data: HashMap<Move, Handle<MoveData>>,

        #[asset(path = "moves", collection(typed, mapped))]
        pub image: HashMap<Move, Handle<Image>>,
    }

    #[derive(Resource, Deref, DerefMut)]
    pub(crate) struct MoveInterfaces(HashMap<Move, Box<dyn MoveComponent>>);

    #[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
    pub(crate) struct MoveData {
        pub(crate) display_name: String,
        pub(crate) spritesheet: Option<AnimationSpritesheet>,
        pub(crate) collider: Option<ColliderShape>,
        #[serde(flatten)]
        #[reflect(ignore)]
        pub(crate) extra_info: HashMap<String, ron::Value>,
    }

    impl MapKey for Move {
        fn from_asset_path(path: &bevy::asset::AssetPath) -> Self {
            short_flight::from_asset_path(path)
        }
    }

    pub struct SpawnMove {
        pub move_id: Move,
        pub parent: Entity,
    }

    impl Command for SpawnMove {
        fn apply(self, world: &mut World) {
            let move_list = world.resource::<MoveList>();
            let Some(handle) = move_list.data.get(&self.move_id) else {
                log::error!("Could not find move data file for {:?}", self.move_id);
                return;
            };

            let move_data = world
                .resource::<Assets<MoveData>>()
                .get(handle)
                .unwrap()
                .clone();

            let bundle = (
                Name::new(move_data.display_name.clone()),
                self.move_id,
                MoveInfo {
                    id: self.move_id,
                    data: handle.clone(),
                },
                Damage(2),
                Transform::default(),
            );
            // world.entity_mut(self.parent).with_child(bundle).id();

            let entity = world.spawn(bundle).insert(ChildOf(self.parent)).id();

            world.resource_scope(|world, mut move_interfaces: Mut<MoveInterfaces>| {
                let Some(interface) = move_interfaces.get_mut(&self.move_id) else {
                    log::error!("No interface found for {:?}", self.move_id);
                    return;
                };
                interface.on_spawn(world, entity, &move_data);
            });
        }
    }
}
