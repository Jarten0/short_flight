use bevy::asset::AssetLoader;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use serde::Deserialize;
use short_flight::animation::AnimationData;
use std::marker::PhantomData;
use thiserror::Error;
pub mod shaymin;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AssetStates>()
            .add_loading_state(
                LoadingState::new(AssetStates::AssetLoading)
                    .continue_to_state(AssetStates::Done)
                    .load_collection::<shaymin::SpritesCollection>(),
            )
            .register_asset_loader::<RonAssetLoader<AnimationData>>(RonAssetLoader::default())
            // .add_systems(OnEnter(AssetStates::Next), start_background_audio)
           ;
    }
}

#[derive(Debug, States, PartialEq, Eq, Default, Hash, Clone)]
enum AssetStates {
    #[default]
    AssetLoading,
    Done,
}

// /// This system runs in MyStates::Next. Thus, AudioAssets is available as a resource
// /// and the contained handle is done loading.
// fn start_background_audio(mut commands: Commands, audio_assets: Res<AudioAssets>) {
//     commands.spawn((
//         AudioPlayer(audio_assets.background.clone()),
//         PlaybackSettings::LOOP,
//     ));
// }

#[derive(Debug, Default)]
pub(crate) struct RonAssetLoader<T> {
    marker: PhantomData<T>,
}

#[derive(Debug, Error)]
pub(crate) enum RonAssetLoaderError {
    #[error("Could not load RON file: {0}")]
    Io(#[from] std::io::Error),
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

        let deserialized = ron::from_str::<T>(&String::from_utf8(bytes).unwrap())?;

        Ok(deserialized)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}
