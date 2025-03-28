use bevy::asset::saver::{AssetSaver, SavedAsset};
use bevy::asset::{AssetLoader, AsyncWriteExt};
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use serde::{Deserialize, Serialize};
use short_flight::animation::{AnimType, AnimationData};
use std::collections::HashMap;
use std::marker::PhantomData;
use thiserror::Error;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AssetStates>()
        .init_asset::<AnimationSpritesheet>()
            // .add_loading_state(
            //     LoadingState::new(AssetStates::First).continue_to_state(AssetStates::PlayerLoading), // .load_collection::<shaymin::SpritesCollection>(),
            // )
            // .init_asset::<AnimationData>()
            // .register_asset_loader::<RonAssetLoader<AnimationData>>(RonAssetLoader::default())
            ;
    }
}

#[derive(Debug, States, PartialEq, Eq, Default, Hash, Clone)]
pub enum AssetStates {
    Retry,
    #[default]
    // First,
    PlayerLoading,
    NPCsLoading,
    Done,
}

#[derive(Debug, Default)]
pub(crate) struct RonAssetLoader<T> {
    marker: PhantomData<T>,
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

impl<T> AssetSaver for RonAssetLoader<T>
where
    T: bevy::prelude::Asset + for<'a> serde::Deserialize<'a> + serde::Serialize,
{
    type Asset = T;
    type Settings = ();
    type OutputLoader = Self;
    type Error = RonAssetLoaderError;

    async fn save(
        &self,
        writer: &mut bevy::asset::io::Writer,
        asset: SavedAsset<'_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<(), RonAssetLoaderError> {
        let buf = ron::to_string(asset.get())?;
        writer.write_all(buf.as_bytes()).await?;
        Ok(())
    }
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

        let deserialized = ron::from_str::<T>(&String::from_utf8(bytes).unwrap())?;

        Ok(deserialized)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

#[derive(Debug, Default, Asset, Reflect, Deserialize, Serialize)]
pub(super) struct AnimationAssets(pub HashMap<AnimType, AnimationData>);

/// An ordered layout of corresponding animation data for a given spritesheet
#[derive(Debug, Default, Asset, Reflect, Deserialize, Serialize)]
pub(super) struct AnimationSpritesheet {
    pub animations: Vec<AnimType>,
    pub sprite_size: u32,
    #[serde(skip)]
    pub data: AnimationAssets,
    #[serde(skip)]
    pub atlas: Option<TextureAtlasLayout>,
}

impl std::ops::Index<AnimType> for AnimationSpritesheet {
    type Output = AnimationData;

    fn index(&self, index: AnimType) -> &Self::Output {
        &self.data.0[&index]
    }
}

impl AnimationSpritesheet {
    fn get_texture_atlas(&self) -> TextureAtlasLayout {
        let max_items = self
            .data
            .0
            .iter()
            .map(|data| data.1.frames)
            .max()
            .unwrap_or(0);
        TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            max_items,
            self.animations.len() as u32,
            None,
            None,
        )
    }
}
