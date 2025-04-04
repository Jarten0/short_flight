use bevy::prelude::*;
use enum_iterator::Sequence;
use prelude::MoveComponent;
use serde::{Deserialize, Serialize};

mod prelude {
    pub use super::interfaces::MoveComponent;
    pub use bevy::prelude::*;
}

mod magical_leaf;
mod tackle;

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

fn register_component(input: Move) -> for<'a> fn(&'a mut bevy::prelude::App) {
    match input {
        Move::Void => void,
        Move::Tackle => tackle::Tackle::build,
        Move::MagicalLeaf => magical_leaf::MagicalLeaf::build,
        // _ => void,
    }
}

fn void(_: &mut App) {}

pub mod interfaces {
    use super::prelude::*;
    use super::register_component;
    use super::Move;
    use crate::assets::AnimationSpritesheet;
    use bevy::utils::hashbrown::HashMap;
    use bevy_asset_loader::asset_collection::AssetCollection;
    use bevy_asset_loader::mapped::MapKey;
    use serde::{Deserialize, Serialize};
    use short_flight::collision::ColliderShape;

    /// This component operates with the move system
    pub trait MoveComponent: Reflect {
        fn build(app: &mut App)
        where
            Self: Sized;
    }

    pub struct MovePlugin;

    impl Plugin for MovePlugin {
        fn build(&self, app: &mut App) {
            for input in enum_iterator::all::<Move>() {
                register_component(input)(app);
            }
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

    #[derive(Debug, Asset, Reflect, Serialize, Deserialize, Clone, Default)]
    pub(crate) struct MoveData {
        pub(crate) display_name: String,
        pub(crate) spritesheet: AnimationSpritesheet,
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

    pub struct SpawnMove<T: MoveComponent + Component> {
        pub(crate) move_id: Move,
        pub(crate) move_: T,
        pub(crate) parent: Entity,
    }

    impl<T> Command for SpawnMove<T>
    where
        T: Component,
    {
        fn apply(self, world: &mut World) {
            let move_list = world.resource::<MoveList>();
            let Some(handle) = move_list.data.get(&self.move_id) else {
                log::error!("Could not find move data file for {:?}", self.move_id);
                return;
            };

            let move_data = world.resource::<Assets<MoveData>>().get(handle).unwrap();

            world
                .spawn((
                    Name::new(move_data.display_name.clone()),
                    self.move_id,
                    self.move_,
                    MoveInfo {
                        id: self.move_id,
                        data: handle.clone(),
                    },
                ))
                .set_parent(self.parent);
        }
    }
}
