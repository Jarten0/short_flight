use crate::animation::{AnimType, AnimationData};
use crate::moves::interfaces::MoveData;
use crate::projectile::interfaces::ProjectileData;
use crate::{ldtk, moves, npc, projectile, shaymin};
use bevy::asset::AssetLoader;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

pub fn loaded(load: Res<State<ShortFlightLoadingState>>) -> bool {
    load.done()
}
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
            .init_asset::<ProjectileData>()
            .register_asset_reflect::<ProjectileData>()
            .register_asset_loader(RonAssetLoader::<ProjectileData>::with_extension(&[
                "proj.ron",
            ]))
            .add_loading_state(
                LoadingState::new(ShortFlightLoadingState::First)
                    .load_collection::<ldtk::MapAssets>()
                    .load_collection::<shaymin::assets::ShayminAssets>()
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
                    .load_collection::<projectile::interfaces::ProjectileCatalog>()
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

impl ShortFlightLoadingState {
    pub fn done(&self) -> bool {
        match self {
            ShortFlightLoadingState::Done => true,
            ShortFlightLoadingState::FailState => true,
            _ => false,
        }
    }
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
pub(crate) struct AnimationAssets(pub HashMap<AnimType, AnimationData>);

/// An ordered layout of corresponding animation data for a given spritesheet
#[derive(Debug, Default, Asset, Reflect, Deserialize, Serialize, Clone)]
pub(crate) struct AnimationSpritesheet {
    pub animations: Vec<AnimType>,
    pub sprite_size: UVec2,
    pub data: AnimationAssets,
    #[serde(skip)]
    pub max_frames: u32,
    #[serde(skip)]
    pub total_variants: u32,
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
        let mut spritesheet = Self {
            animations: data.iter().map(|value| value.0).collect(),
            data: AnimationAssets(data.into_iter().collect()),
            texture: Some(texture),
            sprite_size,
            ..Default::default()
        };

        spritesheet.atlas = Some(asset_server.add(spritesheet.get_atlas_layout()));

        spritesheet
    }
}

impl std::ops::Index<AnimType> for AnimationSpritesheet {
    type Output = AnimationData;

    fn index(&self, index: AnimType) -> &Self::Output {
        &self.data.0[&index]
    }
}

impl AnimationSpritesheet {
    pub fn get_atlas_layout(&mut self) -> TextureAtlasLayout {
        self.max_frames = self
            .data
            .0
            .iter()
            .map(|(_, data)| data.frames)
            .max()
            .unwrap_or(0);
        self.total_variants = self
            .data
            .0
            .iter()
            .map(|(_, data)| data.direction_label.directional_sprite_count())
            .sum();
        TextureAtlasLayout::from_grid(
            self.sprite_size,
            self.max_frames,
            self.total_variants,
            None,
            None,
        )
    }
}
