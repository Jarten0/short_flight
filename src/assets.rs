use crate::moves::interfaces::MoveData;
use crate::{ldtk, moves, npc, player};
use bevy::asset::AssetLoader;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_asset_loader::prelude::*;
use serde::{Deserialize, Serialize};
use short_flight::animation::{AnimType, AnimationData};
use std::marker::PhantomData;
use thiserror::Error;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ShortFlightLoadingState>()
            .init_asset::<AnimationSpritesheet>()
            .init_asset::<npc::file::NPCData>()
            .register_asset_loader(RonAssetLoader::<npc::file::NPCData>::with_extension(&[
                "npc.ron",
            ]))
            .init_asset::<AnimationAssets>()
            .register_asset_loader(RonAssetLoader::<AnimationAssets>::with_extension(&[
                "anim.ron",
            ]))
            .init_asset::<MoveData>()
            .register_asset_loader(RonAssetLoader::<MoveData>::with_extension(&["move.ron"]))
            .add_loading_state(
                LoadingState::new(ShortFlightLoadingState::First)
                    .load_collection::<ldtk::MapAssets>()
                    .load_collection::<player::assets::ShayminAssets>()
                    .on_failure_continue_to_state(ShortFlightLoadingState::FailState)
                    .continue_to_state(ShortFlightLoadingState::PlayerLoading),
            )
            .add_loading_state(
                LoadingState::new(ShortFlightLoadingState::PlayerLoading)
                    .on_failure_continue_to_state(ShortFlightLoadingState::FailState)
                    .continue_to_state(ShortFlightLoadingState::LoadNPCAssets),
            )
            .add_loading_state(
                LoadingState::new(ShortFlightLoadingState::LoadNPCAssets)
                    .load_collection::<npc::file::NPCAlmanac>()
                    .load_collection::<moves::interfaces::MoveList>()
                    .on_failure_continue_to_state(ShortFlightLoadingState::FailState)
                    .continue_to_state(ShortFlightLoadingState::SpawnWithAssets),
            )
            .add_loading_state(
                LoadingState::new(ShortFlightLoadingState::SpawnWithAssets)
                    .on_failure_continue_to_state(ShortFlightLoadingState::FailState)
                    .continue_to_state(ShortFlightLoadingState::Done),
            )
            .add_loading_state(LoadingState::new(ShortFlightLoadingState::FailState));
        // .add_loading_state(LoadingState::new(ShortFlightLoadingState::Done))
    }
}

#[derive(Debug, States, PartialEq, Eq, Default, Hash, Clone)]
pub enum ShortFlightLoadingState {
    FailState,
    Retry,
    #[default]
    First,
    PlayerLoading,
    LoadNPCAssets,
    SpawnWithAssets,
    Done,
}

#[derive(Debug)]
pub(crate) struct RonAssetLoader<T> {
    marker: PhantomData<T>,
    extension: &'static [&'static str],
}

impl<T> RonAssetLoader<T> {
    pub fn with_extension(extension: &'static [&'static str]) -> RonAssetLoader<T> {
        Self {
            marker: Default::default(),
            extension,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum RonAssetLoaderError {
    #[error("Could not load RON file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not serialize RON file: {0}")]
    Serialize(#[from] ron::error::Error),
    #[error("Could not deserialize RON file: {0}")]
    Deserialize(#[from] ron::error::SpannedError),
}

impl<T> AssetLoader for RonAssetLoader<T>
where
    T: Asset + for<'a> Deserialize<'a>,
{
    type Asset = T;
    type Settings = ();
    type Error = RonAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes).await?;

        Ok(ron::de::from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        self.extension
    }
}

#[derive(Debug, Default, Asset, Reflect, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub(super) struct AnimationAssets(pub HashMap<AnimType, AnimationData>);

/// An ordered layout of corresponding animation data for a given spritesheet
#[derive(Debug, Default, Asset, Reflect, Deserialize, Serialize, Clone)]
pub(super) struct AnimationSpritesheet {
    pub animations: Vec<AnimType>,
    pub sprite_size: UVec2,
    pub data: AnimationAssets,
    #[serde(skip)]
    pub max_items: u32,
    #[serde(skip)]
    pub atlas: Option<Handle<TextureAtlasLayout>>,
    #[serde(skip)]
    pub texture: Option<Handle<Image>>,
}

impl AnimationSpritesheet {
    pub fn new(
        data: Vec<(AnimType, AnimationData)>,
        sprite_size: UVec2,
        texture: Handle<Image>,
        asset_server: &AssetServer,
    ) -> Self {
        let mut s = Self {
            animations: data.iter().map(|value| value.0).collect(),
            sprite_size,
            data: AnimationAssets(data.into_iter().collect()),
            max_items: 0,
            atlas: None,
            texture: Some(texture),
        };

        s.atlas = Some(asset_server.add(dbg!(s.get_texture_atlas())));

        s
    }
}

impl std::ops::Index<AnimType> for AnimationSpritesheet {
    type Output = AnimationData;

    fn index(&self, index: AnimType) -> &Self::Output {
        &self.data.0[&index]
    }
}

impl AnimationSpritesheet {
    pub fn get_texture_atlas(&mut self) -> TextureAtlasLayout {
        self.max_items = self
            .data
            .0
            .iter()
            .map(|data| data.1.frames)
            .max()
            .unwrap_or(0);
        TextureAtlasLayout::from_grid(
            self.sprite_size,
            self.max_items,
            self.animations.len() as u32,
            None,
            None,
        )
    }
}
