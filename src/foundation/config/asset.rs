use crate::foundation::config::{
    ConfigInputPolicy, ConfigLoadError, ConfigPath, ConfigRuntime, ConfigState,
    NoConfigValidationError, Validate, apply_config_runtime, load_config_bytes,
    load_validated_config_bytes, resolve_config_asset,
};
use bevy::asset::{Asset, AssetApp, AssetLoader, AssetMut, Assets, LoadContext, io::Reader};
use bevy::prelude::{
    App, AssetId, AssetServer, Commands, Handle, IntoScheduleConfigs, Plugin, Res, Resource,
    Startup, Update,
};
use bevy::reflect::TypePath;
use std::any::type_name;
use std::marker::PhantomData;

pub trait ConfigFormatLoader<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn load(&self, bytes: &[u8]) -> Result<T, Self::Error>;

    fn extensions(&self) -> &[&str];
}

#[doc(hidden)]
#[derive(Asset)]
pub struct ConfigAsset<T: Send + Sync + 'static> {
    value: T,
}

impl<T: Send + Sync + 'static> ConfigAsset<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self { value }
    }

    pub(crate) const fn value(&self) -> &T {
        &self.value
    }

    pub(crate) const fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> TypePath for ConfigAsset<T>
where
    T: Send + Sync + 'static,
{
    fn type_path() -> &'static str {
        type_name::<Self>()
    }

    fn short_type_path() -> &'static str {
        type_name::<Self>()
    }

    fn type_ident() -> Option<&'static str> {
        Some("ConfigAsset")
    }

    fn crate_name() -> Option<&'static str> {
        Some(env!("CARGO_CRATE_NAME"))
    }

    fn module_path() -> Option<&'static str> {
        Some(module_path!())
    }
}

pub struct ConfigAssetLoader<T, F, V> {
    format_loader: F,
    validation: V,
    _marker: PhantomData<fn() -> T>,
}

impl<T, F, V> Default for ConfigAssetLoader<T, F, V>
where
    F: Default,
    V: Default,
{
    fn default() -> Self {
        Self {
            format_loader: F::default(),
            validation: V::default(),
            _marker: PhantomData,
        }
    }
}

impl<T, F, V> TypePath for ConfigAssetLoader<T, F, V>
where
    T: 'static,
    F: 'static,
    V: 'static,
{
    fn type_path() -> &'static str {
        type_name::<Self>()
    }

    fn short_type_path() -> &'static str {
        type_name::<Self>()
    }

    fn type_ident() -> Option<&'static str> {
        Some("ConfigAssetLoader")
    }

    fn crate_name() -> Option<&'static str> {
        Some(env!("CARGO_CRATE_NAME"))
    }

    fn module_path() -> Option<&'static str> {
        Some(module_path!())
    }
}

impl<T, F, V> AssetLoader for ConfigAssetLoader<T, F, V>
where
    T: Send + Sync + 'static,
    F: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
{
    type Asset = ConfigAsset<T>;
    type Settings = ();
    type Error = ConfigLoadError<F::Error, V::Error>;

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

        self.validation
            .load::<F>(&bytes, path)
            .map(ConfigAsset::new)
    }

    fn extensions(&self) -> &[&str] {
        self.format_loader.extensions()
    }
}

pub trait ConfigValidation<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn load<F>(
        &self,
        bytes: &[u8],
        path: String,
    ) -> Result<T, ConfigLoadError<F::Error, Self::Error>>
    where
        F: ConfigFormatLoader<T>;
}

#[derive(Default)]
pub struct NoConfigValidation;

impl<T> ConfigValidation<T> for NoConfigValidation {
    type Error = NoConfigValidationError;

    fn load<F>(
        &self,
        bytes: &[u8],
        path: String,
    ) -> Result<T, ConfigLoadError<F::Error, Self::Error>>
    where
        F: ConfigFormatLoader<T>,
    {
        load_config_bytes::<T, F>(bytes, path)
    }
}

#[derive(Default)]
pub struct ValidateConfigValidation;

impl<T> ConfigValidation<T> for ValidateConfigValidation
where
    T: Validate,
{
    type Error = T::Error;

    fn load<F>(
        &self,
        bytes: &[u8],
        path: String,
    ) -> Result<T, ConfigLoadError<F::Error, Self::Error>>
    where
        F: ConfigFormatLoader<T>,
    {
        load_validated_config_bytes::<T, F>(bytes, path)
    }
}

#[derive(Default)]
pub struct NoConfigRuntime;

#[derive(Default)]
pub struct ApplyConfigRuntime;

pub struct ConfigAssetPlugin<T, L, V = NoConfigValidation, R = NoConfigRuntime> {
    path: ConfigPath,
    input_policy: ConfigInputPolicy<T>,
    _marker: PhantomData<fn() -> T>,
    _loader: PhantomData<fn() -> L>,
    _validation: PhantomData<fn() -> V>,
    _runtime: PhantomData<fn() -> R>,
}

impl<T, L> ConfigAssetPlugin<T, L, NoConfigValidation, NoConfigRuntime> {
    pub const fn new(path: ConfigPath) -> Self {
        Self {
            path,
            input_policy: ConfigInputPolicy::required(),
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _runtime: PhantomData,
        }
    }
}

impl<T, L, V, R> ConfigAssetPlugin<T, L, V, R> {
    pub const fn with_validation(self) -> ConfigAssetPlugin<T, L, ValidateConfigValidation, R>
    where
        T: Validate,
    {
        ConfigAssetPlugin {
            path: self.path,
            input_policy: self.input_policy,
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _runtime: PhantomData,
        }
    }

    pub const fn with_runtime(self) -> ConfigAssetPlugin<T, L, V, ApplyConfigRuntime>
    where
        T: ConfigRuntime,
    {
        ConfigAssetPlugin {
            path: self.path,
            input_policy: self.input_policy,
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _runtime: PhantomData,
        }
    }

    pub const fn with_input_policy(mut self, input_policy: ConfigInputPolicy<T>) -> Self {
        self.input_policy = input_policy;
        self
    }

    pub const fn fallback_with(self, default: fn() -> T) -> Self {
        self.with_input_policy(ConfigInputPolicy::fallback_with(default))
    }
}

impl<T, L, V, R> ConfigAssetPlugin<T, L, V, R>
where
    T: Default,
{
    pub fn fallback_to_default(self) -> Self {
        self.with_input_policy(ConfigInputPolicy::fallback_to_default())
    }
}

#[derive(Resource)]
pub struct ConfigHandle<T: Send + Sync + 'static> {
    path: ConfigPath,
    handle: Handle<ConfigAsset<T>>,
}

impl<T: Send + Sync + 'static> ConfigHandle<T> {
    pub const fn path(&self) -> ConfigPath {
        self.path
    }

    pub(crate) fn id(&self) -> AssetId<ConfigAsset<T>> {
        self.handle.id()
    }

    pub(crate) fn get<'a>(&self, assets: &'a Assets<ConfigAsset<T>>) -> Option<&'a T> {
        assets.get(self.id()).map(ConfigAsset::value)
    }

    pub(crate) fn get_mut<'a>(
        &self,
        assets: &'a mut Assets<ConfigAsset<T>>,
    ) -> Option<AssetMut<'a, ConfigAsset<T>>> {
        assets.get_mut(self.id())
    }
}

impl<T, L, V> Plugin for ConfigAssetPlugin<T, L, V, NoConfigRuntime>
where
    T: Clone + Resource,
    ConfigInputPolicy<T>: Send + Sync + 'static,
    L: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
{
    fn build(&self, app: &mut App) {
        let path = self.path;
        let input_policy = self.input_policy;

        build_config_asset_plugin_base::<T, L, V>(app, path, input_policy);
    }
}

impl<T, L, V> Plugin for ConfigAssetPlugin<T, L, V, ApplyConfigRuntime>
where
    T: Clone + ConfigRuntime + Resource,
    ConfigInputPolicy<T>: Send + Sync + 'static,
    L: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
{
    fn build(&self, app: &mut App) {
        let path = self.path;
        let input_policy = self.input_policy;

        build_config_asset_plugin_base::<T, L, V>(app, path, input_policy);
        app.init_resource::<ConfigState<T::Runtime>>().add_systems(
            Update,
            apply_config_runtime::<T>.after(resolve_config_asset::<T>),
        );
    }
}

fn build_config_asset_plugin_base<T, L, V>(
    app: &mut App,
    path: ConfigPath,
    input_policy: ConfigInputPolicy<T>,
) where
    T: Clone + Resource,
    ConfigInputPolicy<T>: Send + Sync + 'static,
    L: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
{
    app.init_asset::<ConfigAsset<T>>()
        .insert_resource(input_policy)
        .init_resource::<ConfigState<T>>()
        .register_asset_loader(ConfigAssetLoader::<T, L, V>::default())
        .add_systems(Update, resolve_config_asset::<T>)
        .add_systems(
            Startup,
            move |mut commands: Commands, asset_server: Res<AssetServer>| {
                let handle = asset_server.load(path.asset_path());

                commands.insert_resource(ConfigHandle::<T> { path, handle });
            },
        );
}
