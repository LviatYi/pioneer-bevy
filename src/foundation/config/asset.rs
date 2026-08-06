use super::config_context::{
    ConfigState, InitialLoadPolicy, TransformToRuntimeResourceConfig, ValidateConfig,
    resolve_config_output,
};
use super::error::{ConfigLoadError, NoConfigValidationError};
use super::path::ConfigPath;
use bevy::asset::{Asset, AssetApp, AssetLoader, Assets, LoadContext, io::Reader};
use bevy::prelude::{
    App, AssetId, AssetServer, Commands, Handle, Plugin, Res, Resource, Startup, Update,
};
use bevy::reflect::TypePath;
use std::any::type_name;
use std::convert::Infallible;
use std::marker::PhantomData;

//region Loader

pub trait ConfigFormatLoader<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn load(&self, bytes: &[u8]) -> Result<T, Self::Error>;

    fn extensions(&self) -> &[&str];
}

#[doc(hidden)]
#[derive(Asset)]
pub(super) struct ConfigAsset<T: Send + Sync + 'static> {
    value: T,
}

impl<T: Send + Sync + 'static> ConfigAsset<T> {
    pub(super) const fn new(value: T) -> Self {
        Self { value }
    }

    pub(super) const fn value(&self) -> &T {
        &self.value
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

struct ConfigAssetLoader<T, F, V> {
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

        let config = self
            .format_loader
            .load(&bytes)
            .map_err(|source| ConfigLoadError::Format {
                path: path.clone(),
                source,
            })?;
        self.validation
            .validate(&config)
            .map_err(|source| ConfigLoadError::Validation { path, source })?;

        Ok(ConfigAsset::new(config))
    }

    fn extensions(&self) -> &[&str] {
        self.format_loader.extensions()
    }
}

//endregion

//region Validation

pub trait ConfigValidation<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self, config: &T) -> Result<(), Self::Error>;
}

#[derive(Default)]
pub struct NoConfigValidation;

impl<T> ConfigValidation<T> for NoConfigValidation {
    type Error = NoConfigValidationError;

    fn validate(&self, _config: &T) -> Result<(), Self::Error> {
        Ok(())
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

    fn validate(&self, config: &T) -> Result<(), Self::Error> {
        config.validate()
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

pub(super) trait ConfigOutput<T>: Send + Sync + 'static {
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
pub(super) struct ConfigHandle<T: Send + Sync + 'static> {
    path: ConfigPath,
    handle: Handle<ConfigAsset<T>>,
}

impl<T: Send + Sync + 'static> ConfigHandle<T> {
    pub(super) const fn path(&self) -> ConfigPath {
        self.path
    }

    pub(super) fn id(&self) -> AssetId<ConfigAsset<T>> {
        self.handle.id()
    }

    pub(super) fn get<'a>(&self, assets: &'a Assets<ConfigAsset<T>>) -> Option<&'a T> {
        assets.get(self.id()).map(ConfigAsset::value)
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

        app.init_asset::<ConfigAsset<T>>()
            .insert_resource(input_policy)
            .register_asset_loader(ConfigAssetLoader::<T, L, V>::default())
            .add_systems(
                Startup,
                move |mut commands: Commands, asset_server: Res<AssetServer>| {
                    let handle = asset_server.load(path.asset_path());

                    commands.insert_resource(ConfigHandle::<T> { path, handle });
                },
            )
            .init_resource::<ConfigState<O::Output>>()
            .add_systems(Update, resolve_config_output::<T, O>);
    }
}

#[cfg(test)]
mod tests {
    use super::super::ron::RonConfigPlugin;
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::prelude::*;
    use serde::Deserialize;
    use std::convert::Infallible;
    use std::error::Error;
    use std::fmt::{Display, Formatter};

    const TEST_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");

    #[derive(Debug, Default, Deserialize, PartialEq)]
    struct TestConfig {
        base_cell_size_cm: u32,
        building_cell_size_cm: u32,
    }

    impl TransformToRuntimeResourceConfig for TestConfig {
        type Runtime = TestRuntime;
        type Error = Infallible;

        fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error> {
            Ok(TestRuntime)
        }
    }

    #[derive(Resource)]
    struct TestRuntime;

    #[derive(Debug)]
    struct ValidatedTestConfig {
        value: u32,
    }

    impl ValidateConfig for ValidatedTestConfig {
        type Error = TestConfigValidationError;

        fn validate(&self) -> Result<(), Self::Error> {
            if self.value == 0 {
                return Err(TestConfigValidationError);
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    struct TestConfigValidationError;

    impl Display for TestConfigValidationError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("value must be greater than 0")
        }
    }

    impl Error for TestConfigValidationError {}

    #[test]
    fn validation_reports_validation_errors() {
        let error = WithValidation
            .validate(&ValidatedTestConfig { value: 0 })
            .unwrap_err();

        assert!(matches!(error, TestConfigValidationError));
    }

    #[test]
    fn validation_accepts_valid_configs() {
        let config = ValidatedTestConfig { value: 7 };

        assert!(WithValidation.validate(&config).is_ok());
    }

    #[test]
    fn no_validation_accepts_configs_without_validation() {
        let config = ValidatedTestConfig { value: 0 };

        assert!(NoConfigValidation.validate(&config).is_ok());
    }

    #[test]
    fn runtime_plugin_registers_config_asset() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<TestConfig>::new(TEST_CONFIG_PATH).with_runtime_transformer(),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<TestConfig>>();

        assert_eq!(handle.path(), TEST_CONFIG_PATH);
        assert!(
            app.world()
                .get_resource::<ConfigState<TestRuntime>>()
                .is_some()
        );
        assert!(
            app.world()
                .get_resource::<ConfigState<TestConfig>>()
                .is_none()
        );
    }

    #[test]
    fn config_handle_reads_registered_asset() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<TestConfig>::new(TEST_CONFIG_PATH).with_runtime_transformer(),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<TestConfig>>();
        let id = handle.id();
        let assets = app
            .world_mut()
            .resource_mut::<Assets<ConfigAsset<TestConfig>>>();
        assets
            .into_inner()
            .insert(id, ConfigAsset::new(TestConfig::default()))
            .unwrap();

        let handle = app.world().resource::<ConfigHandle<TestConfig>>();
        let assets = app.world().resource::<Assets<ConfigAsset<TestConfig>>>();

        assert_eq!(handle.get(assets), Some(&TestConfig::default()));
    }
}
