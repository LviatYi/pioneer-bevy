use crate::foundation::config::{
    ConfigLoadError, ConfigPath, ConfigState, InitialLoadPolicy, NoConfigValidationError,
    TransformToRuntimeResourceConfig, ValidateConfig, load_config_bytes,
    load_validated_config_bytes, resolve_config_output,
};
use bevy::asset::{Asset, AssetApp, AssetLoader, AssetMut, Assets, LoadContext, io::Reader};
use bevy::prelude::{
    App, AssetId, AssetServer, Commands, Handle, Plugin, Res, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use std::any::type_name;
use std::convert::Infallible;
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

//region Validation

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
#[doc(hidden)]
pub struct WithValidation;

impl<T> ConfigValidation<T> for WithValidation
where
    T: ValidateConfig,
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

//endregion

//region Config Output

#[derive(Default)]
#[doc(hidden)]
pub struct RawResourceOutput;

#[derive(Default)]
#[doc(hidden)]
pub struct RuntimeResourceOutput;

pub(crate) trait ConfigOutput<T>: Send + Sync + 'static {
    type Output: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error>;
}

impl<T> ConfigOutput<T> for RawResourceOutput
where
    T: Clone + Resource,
{
    type Output = T;
    type Error = Infallible;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error> {
        Ok(raw.clone())
    }
}

impl<T> ConfigOutput<T> for RuntimeResourceOutput
where
    T: TransformToRuntimeResourceConfig,
{
    type Output = T::Runtime;
    type Error = T::Error;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error> {
        raw.transform_to_runtime()
    }
}

//endregion

pub struct ConfigPlugin<T, L, V = NoConfigValidation, O = RawResourceOutput> {
    path: ConfigPath,
    input_policy: InitialLoadPolicy<T>,
    _marker: PhantomData<fn() -> T>,
    _loader: PhantomData<fn() -> L>,
    _validation: PhantomData<fn() -> V>,
    _output: PhantomData<fn() -> O>,
}

impl<T, L> ConfigPlugin<T, L, NoConfigValidation, RawResourceOutput> {
    pub const fn new(path: ConfigPath) -> Self {
        Self {
            path,
            input_policy: InitialLoadPolicy::required(),
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _output: PhantomData,
        }
    }
}

impl<T, L, V, O> ConfigPlugin<T, L, V, O> {
    pub const fn with_validation(self) -> ConfigPlugin<T, L, WithValidation, O>
    where
        T: ValidateConfig,
    {
        ConfigPlugin {
            path: self.path,
            input_policy: self.input_policy,
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _output: PhantomData,
        }
    }

    pub const fn with_input_policy(mut self, input_policy: InitialLoadPolicy<T>) -> Self {
        self.input_policy = input_policy;
        self
    }

    pub const fn fallback_with(self, default: fn() -> T) -> Self {
        self.with_input_policy(InitialLoadPolicy::fallback_with(default))
    }
}

impl<T, L, V> ConfigPlugin<T, L, V, RawResourceOutput> {
    pub const fn with_runtime_transformer(self) -> ConfigPlugin<T, L, V, RuntimeResourceOutput>
    where
        T: TransformToRuntimeResourceConfig,
    {
        ConfigPlugin {
            path: self.path,
            input_policy: self.input_policy,
            _marker: PhantomData,
            _loader: PhantomData,
            _validation: PhantomData,
            _output: PhantomData,
        }
    }
}

impl<T, L, V, O> ConfigPlugin<T, L, V, O>
where
    T: Default,
{
    pub fn fallback_to_default(self) -> Self {
        self.with_input_policy(InitialLoadPolicy::fallback_to_default())
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

impl<T, L, V, O> Plugin for ConfigPlugin<T, L, V, O>
where
    T: Send + Sync + 'static,
    InitialLoadPolicy<T>: Send + Sync + 'static,
    L: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
    O: ConfigOutput<T>,
{
    fn build(&self, app: &mut App) {
        let path = self.path;
        let input_policy = self.input_policy;

        register_config_asset_source::<T, L, V>(app, path, input_policy);
        app.init_resource::<ConfigState<O::Output>>()
            .add_systems(Update, resolve_config_output::<T, O>);
    }
}

fn register_config_asset_source<T, L, V>(
    app: &mut App,
    path: ConfigPath,
    input_policy: InitialLoadPolicy<T>,
) where
    T: Send + Sync + 'static,
    InitialLoadPolicy<T>: Send + Sync + 'static,
    L: ConfigFormatLoader<T>,
    V: ConfigValidation<T>,
{
    app.init_asset::<ConfigAsset<T>>()
        .insert_resource(input_policy)
        .register_asset_loader(ConfigAssetLoader::<T, L, V>::default())
        .add_systems(
            Startup,
            move |mut commands: Commands, asset_server: Res<AssetServer>| {
                let handle = asset_server.load(path.asset_path());

                commands.insert_resource(ConfigHandle::<T> { path, handle });
            },
        );
}
