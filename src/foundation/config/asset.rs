use crate::foundation::config::{
    ConfigContext, ConfigLoadError, ConfigPath, ConfigRuntimeState, Validate,
    apply_loaded_config_runtime, load_config_bytes,
};
use bevy::asset::{AssetApp, AssetLoader, AssetMut, Assets, LoadContext, io::Reader};
use bevy::prelude::{
    App, Asset, AssetId, AssetServer, Commands, Handle, Plugin, Res, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use std::marker::PhantomData;

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
    type Error = ConfigLoadError<F::Error, T::Error>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        let path = load_context.path().to_string();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|source| ConfigLoadError::Read {
                path: path.clone(),
                source,
            })?;

        load_config_bytes::<T, F>(&bytes, path)
    }

    fn extensions(&self) -> &[&str] {
        self.format_loader.extensions()
    }
}

pub struct ConfigAssetPlugin<T, L> {
    path: ConfigPath,
    _marker: PhantomData<fn() -> T>,
    _loader: PhantomData<fn() -> L>,
}

impl<T, L> ConfigAssetPlugin<T, L> {
    pub const fn new(path: ConfigPath) -> Self {
        Self {
            path,
            _marker: PhantomData,
            _loader: PhantomData,
        }
    }
}

#[derive(Resource)]
pub struct ConfigHandle<T: Asset> {
    path: ConfigPath,
    handle: Handle<T>,
}

impl<T: Asset> ConfigHandle<T> {
    pub const fn path(&self) -> ConfigPath {
        self.path
    }

    pub fn handle(&self) -> &Handle<T> {
        &self.handle
    }

    pub fn id(&self) -> AssetId<T> {
        self.handle.id()
    }

    pub fn get<'a>(&self, assets: &'a Assets<T>) -> Option<&'a T> {
        assets.get(self.id())
    }

    pub fn get_mut<'a>(&self, assets: &'a mut Assets<T>) -> Option<AssetMut<'a, T>> {
        assets.get_mut(self.id())
    }
}

impl<T, L> Plugin for ConfigAssetPlugin<T, L>
where
    T: ConfigContext,
    L: ConfigFormatLoader<T> + TypePath,
{
    fn build(&self, app: &mut App) {
        let path = self.path;

        app.init_asset::<T>()
            .register_asset_loader(ConfigAssetLoader::<T, L>::default())
            .init_resource::<ConfigRuntimeState<T>>()
            .add_systems(
                Startup,
                move |mut commands: Commands, asset_server: Res<AssetServer>| {
                    let handle = asset_server.load(path.asset_path());

                    commands.insert_resource(ConfigHandle::<T> { path, handle });
                },
            )
            .add_systems(Update, apply_loaded_config_runtime::<T>);
    }
}
