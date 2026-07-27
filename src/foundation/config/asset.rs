use crate::foundation::config::Validate;
use bevy::asset::{AssetApp, AssetLoader, LoadContext, io::Reader};
use bevy::prelude::{App, Asset, Plugin};
use bevy::reflect::TypePath;
use std::marker::PhantomData;
use thiserror::Error;

pub trait ConfigFormatLoader<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn load(&self, bytes: &[u8]) -> Result<T, Self::Error>;

    fn extensions(&self) -> &[&str];
}

#[derive(TypePath)]
pub struct ConfigAssetLoader<T, F> {
    format_loader: F,
    _marker: PhantomData<fn() -> T>,
}

impl<T, F> Default for ConfigAssetLoader<T, F>
where
    F: Default,
{
    fn default() -> Self {
        Self {
            format_loader: F::default(),
            _marker: PhantomData,
        }
    }
}

impl<T, F> AssetLoader for ConfigAssetLoader<T, F>
where
    T: Asset + Validate,
    F: ConfigFormatLoader<T> + TypePath,
{
    type Asset = T;
    type Settings = ();
    type Error = ConfigAssetLoaderError<F::Error, T::Error>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let config =
            self.format_loader
                .load(&bytes)
                .map_err(|source| ConfigAssetLoaderError::Format {
                    path: load_context.path().to_string(),
                    source,
                })?;

        config
            .validate()
            .map_err(|source| ConfigAssetLoaderError::Validation {
                path: load_context.path().to_string(),
                source,
            })?;

        Ok(config)
    }

    fn extensions(&self) -> &[&str] {
        self.format_loader.extensions()
    }
}

#[derive(Debug, Error)]
pub enum ConfigAssetLoaderError<FormatError, ValidationError>
where
    FormatError: std::error::Error + Send + Sync + 'static,
    ValidationError: std::error::Error + Send + Sync + 'static,
{
    #[error("failed to read config asset")]
    Read(#[from] std::io::Error),
    #[error("failed to load config asset `{path}` from its storage format")]
    Format {
        path: String,
        #[source]
        source: FormatError,
    },
    #[error("config asset `{path}` failed validation")]
    Validation {
        path: String,
        #[source]
        source: ValidationError,
    },
}

pub struct ConfigAssetPlugin<T, L> {
    _marker: PhantomData<fn() -> T>,
    _loader: PhantomData<fn() -> L>,
}

impl<T, L> ConfigAssetPlugin<T, L> {
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            _loader: PhantomData,
        }
    }
}

impl<T, L> Default for ConfigAssetPlugin<T, L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, L> Plugin for ConfigAssetPlugin<T, L>
where
    T: Asset + Validate,
    L: ConfigFormatLoader<T> + TypePath,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<T>()
            .register_asset_loader(ConfigAssetLoader::<T, L>::default());
    }
}
