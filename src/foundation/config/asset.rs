use super::error::ConfigLoadError;
use super::output::{
    ConfigOutput, RawResourceOutput, RuntimeResourceOutput, TransformToRuntimeResourceConfig,
};
use super::path::ConfigPath;
use super::validation::{ConfigValidation, NoConfigValidation, ValidateConfig, WithValidation};
use bevy::asset::{
    Asset, AssetApp, AssetEvent, AssetLoadFailedEvent, AssetLoader, Assets, LoadContext, io::Reader,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    App, AssetId, AssetServer, Commands, Handle, MessageReader, Plugin, Res, ResMut, Resource,
    Startup, Update,
};
use bevy::reflect::TypePath;
use std::any::type_name;
use std::marker::PhantomData;
use tracing::warn;
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

//region Lifecycle

#[derive(Resource)]
pub enum InitialLoadPolicy<T> {
    Required,
    FallbackToDefault { default: fn() -> T },
}

impl<T> Copy for InitialLoadPolicy<T> {}

impl<T> Clone for InitialLoadPolicy<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> InitialLoadPolicy<T> {
    pub const fn required() -> Self {
        Self::Required
    }

    pub const fn fallback_with(default: fn() -> T) -> Self {
        Self::FallbackToDefault { default }
    }
}

impl<T> InitialLoadPolicy<T>
where
    T: Default,
{
    pub fn fallback_to_default() -> Self {
        Self::FallbackToDefault {
            default: T::default,
        }
    }
}

impl<T> InitialLoadPolicy<T>
where
    T: Send + Sync + 'static,
{
    fn handle_load_failure(
        self,
        path: ConfigPath,
        failure: &AssetLoadFailedEvent<ConfigAsset<T>>,
    ) -> (T, ConfigSource) {
        match self {
            Self::Required => {
                panic!(
                    "required config `{}` failed to load: {}",
                    failure.path, failure.error
                );
            }
            Self::FallbackToDefault { default } => {
                warn!(
                    config_path = %failure.path,
                    error = %failure.error,
                    "config failed to load; using default fallback"
                );

                (default(), ConfigSource::DefaultFallback(path))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    Asset(ConfigPath),
    DefaultFallback(ConfigPath),
}

impl ConfigSource {
    pub const fn path(self) -> ConfigPath {
        match self {
            Self::Asset(path) | Self::DefaultFallback(path) => path,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigStatus {
    Assetization,
    Resolved { source: ConfigSource },
}

#[derive(Resource)]
pub struct ConfigState<T: Send + Sync + 'static> {
    status: ConfigStatus,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> Default for ConfigState<T> {
    fn default() -> Self {
        Self {
            status: ConfigStatus::Assetization,
            _marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> ConfigState<T> {
    pub const fn is_resolved(&self) -> bool {
        matches!(self.status, ConfigStatus::Resolved { .. })
    }

    pub const fn status(&self) -> &ConfigStatus {
        &self.status
    }

    pub const fn source(&self) -> Option<ConfigSource> {
        match self.status {
            ConfigStatus::Assetization => None,
            ConfigStatus::Resolved { source } => Some(source),
        }
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

/// Bridges Bevy's asset lifecycle into the stable config resource exposed to the world.
///
/// Bevy finalizes asynchronous loads and inserts their assets in `PreUpdate`, while
/// `Assets` publishes the corresponding `Added` or `Modified` messages in `PostUpdate`.
/// This system runs in `Update`, so initial resolution intentionally polls `Assets`
/// directly. A later `Added` message is only a refresh trigger while serving a fallback;
/// otherwise it is the delayed notification for an initial output that was already installed.
#[derive(SystemParam)]
struct ConfigResolutionContext<'w, 's, T, O>
where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    commands: Commands<'w, 's>,
    handle: Res<'w, ConfigHandle<T>>,
    assets: Res<'w, Assets<ConfigAsset<T>>>,
    policy: Res<'w, InitialLoadPolicy<T>>,
    state: ResMut<'w, ConfigState<<O as ConfigOutput<T>>::Output>>,
    asset_events: MessageReader<'w, 's, AssetEvent<ConfigAsset<T>>>,
    failures: MessageReader<'w, 's, AssetLoadFailedEvent<ConfigAsset<T>>>,
}

fn event_requests_refresh<T: Send + Sync + 'static>(
    event: &AssetEvent<ConfigAsset<T>>,
    handle_id: AssetId<ConfigAsset<T>>,
    source: ConfigSource,
) -> bool {
    event.is_modified(handle_id)
        || (event.is_added(handle_id) && matches!(source, ConfigSource::DefaultFallback(_)))
}

fn resolve_config_output<T, O>(mut context: ConfigResolutionContext<T, O>)
where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    if handle_config_load_failure::<T, O>(&mut context) {
        return;
    }

    match context.state.status {
        ConfigStatus::Assetization => resolve_initial_config_output::<T, O>(&mut context),
        ConfigStatus::Resolved { source } => {
            let handle_id = context.handle.id();
            let refresh_requested =
                context
                    .asset_events
                    .read()
                    .fold(false, |refresh_requested, event| {
                        refresh_requested || event_requests_refresh(event, handle_id, source)
                    });

            if refresh_requested {
                refresh_resolved_config_output::<T, O>(&mut context)
            }
        }
    }
}

fn handle_config_load_failure<T, O>(context: &mut ConfigResolutionContext<'_, '_, T, O>) -> bool
where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    let handle_id = context.handle.id();

    for failure in context.failures.read() {
        if failure.id != handle_id {
            continue;
        }

        if context.state.is_resolved() {
            warn!(
                config_path = %failure.path,
                error = %failure.error,
                "config reload failed; keeping last resolved output"
            );
            return true;
        }

        let (config, source) =
            (*context.policy).handle_load_failure(context.handle.path(), failure);
        commit_initial_config_output::<T, O>(
            &mut context.commands,
            &mut context.state,
            &config,
            source,
        );
        return true;
    }

    false
}

fn resolve_initial_config_output<T, O>(context: &mut ConfigResolutionContext<'_, '_, T, O>)
where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    let Some(config) = context.handle.get(&context.assets) else {
        return;
    };

    commit_initial_config_output::<T, O>(
        &mut context.commands,
        &mut context.state,
        config,
        ConfigSource::Asset(context.handle.path()),
    );
}

fn refresh_resolved_config_output<T, O>(context: &mut ConfigResolutionContext<'_, '_, T, O>)
where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    let Some(config) = context.handle.get(&context.assets) else {
        return;
    };

    let source = ConfigSource::Asset(context.handle.path());
    if let Err(error) =
        commit_config_output::<T, O>(&mut context.commands, &mut context.state, config, source)
    {
        warn!(
            config_path = source.path().asset_path(),
            error = %error,
            "config reload failed to produce its resource output; keeping last resolved output"
        );
    }
}

fn commit_initial_config_output<T, O>(
    commands: &mut Commands,
    state: &mut ConfigState<O::Output>,
    config: &T,
    source: ConfigSource,
) where
    O: ConfigOutput<T>,
{
    commit_config_output::<T, O>(commands, state, config, source).unwrap_or_else(|error| {
        panic!(
            "config `{}` failed to produce its resource output: {error}",
            source.path().asset_path()
        )
    });
}

fn commit_config_output<T, O>(
    commands: &mut Commands,
    state: &mut ConfigState<O::Output>,
    config: &T,
    source: ConfigSource,
) -> Result<(), O::Error>
where
    O: ConfigOutput<T>,
{
    let output = O::produce(config)?;

    commands.insert_resource(output);
    state.status = ConfigStatus::Resolved { source };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::ron::RonConfigPlugin;
    use super::*;
    use bevy::asset::{AssetLoadError, AssetLoadFailedEvent, AssetPlugin};
    use bevy::ecs::message::Messages;
    use bevy::prelude::*;
    use serde::Deserialize;
    use std::convert::Infallible;
    use thiserror::Error;

    const TEST_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");
    const MISSING_CONFIG_PATH: ConfigPath = ConfigPath::new("config/missing-config.ron");

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

    #[derive(Debug, Default, Deserialize)]
    struct TransformingTestConfig {
        base_cell_size_cm: u32,
        building_cell_size_cm: u32,
    }

    impl TransformToRuntimeResourceConfig for TransformingTestConfig {
        type Runtime = RuntimeTestConfig;
        type Error = TestTransformError;

        fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error> {
            if self.base_cell_size_cm == 0 {
                return Err(TestTransformError);
            }

            Ok(RuntimeTestConfig {
                base_cells_per_building_cell: self.building_cell_size_cm / self.base_cell_size_cm,
            })
        }
    }

    #[derive(Debug, Error)]
    #[error("base cell size must be greater than zero")]
    struct TestTransformError;

    #[derive(Debug, Eq, PartialEq, Resource)]
    struct RuntimeTestConfig {
        base_cells_per_building_cell: u32,
    }

    #[derive(Clone, Debug, Deserialize, Resource)]
    struct RawTestConfig {
        base_cell_size_cm: u32,
        building_cell_size_cm: u32,
    }

    #[test]
    fn added_asset_requests_refresh_only_while_serving_fallback() {
        let handle_id = AssetId::<ConfigAsset<TestConfig>>::invalid();
        let event = AssetEvent::Added { id: handle_id };

        assert!(!event_requests_refresh(
            &event,
            handle_id,
            ConfigSource::Asset(TEST_CONFIG_PATH)
        ));
        assert!(event_requests_refresh(
            &event,
            handle_id,
            ConfigSource::DefaultFallback(TEST_CONFIG_PATH)
        ));
    }

    #[test]
    fn modified_asset_always_requests_refresh() {
        let handle_id = AssetId::<ConfigAsset<TestConfig>>::invalid();
        let event = AssetEvent::Modified { id: handle_id };

        assert!(event_requests_refresh(
            &event,
            handle_id,
            ConfigSource::Asset(TEST_CONFIG_PATH)
        ));
        assert!(event_requests_refresh(
            &event,
            handle_id,
            ConfigSource::DefaultFallback(TEST_CONFIG_PATH)
        ));
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

    #[test]
    fn loaded_config_is_transformed_into_runtime_resource() {
        let mut app = runtime_config_app(TEST_CONFIG_PATH);
        app.update();

        let handle = app
            .world()
            .resource::<ConfigHandle<TransformingTestConfig>>();
        let id = handle.id();
        let assets = app
            .world_mut()
            .resource_mut::<Assets<ConfigAsset<TransformingTestConfig>>>();
        assets
            .into_inner()
            .insert(
                id,
                ConfigAsset::new(TransformingTestConfig {
                    base_cell_size_cm: 5,
                    building_cell_size_cm: 50,
                }),
            )
            .unwrap();

        app.update();

        let runtime = app.world().resource::<RuntimeTestConfig>();
        let state = app.world().resource::<ConfigState<RuntimeTestConfig>>();

        assert_eq!(runtime.base_cells_per_building_cell, 10);
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::Asset(TEST_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn fallback_config_is_transformed_when_asset_load_fails() {
        let mut app = runtime_config_app(MISSING_CONFIG_PATH);
        app.update();

        write_load_failure::<TransformingTestConfig>(&mut app, MISSING_CONFIG_PATH);
        app.update();

        let runtime = app.world().resource::<RuntimeTestConfig>();
        let state = app.world().resource::<ConfigState<RuntimeTestConfig>>();

        assert_eq!(runtime.base_cells_per_building_cell, 10);
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::DefaultFallback(MISSING_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn fallback_raw_config_resolves_raw_config_resource() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<RawTestConfig>::new(MISSING_CONFIG_PATH).fallback_with(|| {
                RawTestConfig {
                    base_cell_size_cm: 5,
                    building_cell_size_cm: 50,
                }
            }),
        ));
        app.update();

        write_load_failure::<RawTestConfig>(&mut app, MISSING_CONFIG_PATH);
        app.update();

        let config = app.world().resource::<RawTestConfig>();
        let state = app.world().resource::<ConfigState<RawTestConfig>>();

        assert_eq!(config.base_cell_size_cm, 5);
        assert_eq!(config.building_cell_size_cm, 50);
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::DefaultFallback(MISSING_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn modified_config_asset_updates_runtime_resource() {
        let mut app = resolved_runtime_config_app();
        replace_target_config(
            &mut app,
            TransformingTestConfig {
                base_cell_size_cm: 5,
                building_cell_size_cm: 100,
            },
        );

        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<RuntimeTestConfig>(),
            &RuntimeTestConfig {
                base_cells_per_building_cell: 20
            }
        );
    }

    #[test]
    fn modified_unrelated_config_asset_is_ignored() {
        let mut app = resolved_runtime_config_app();
        let other_handle = app
            .world_mut()
            .resource_mut::<Assets<ConfigAsset<TransformingTestConfig>>>()
            .add(ConfigAsset::new(TransformingTestConfig {
                base_cell_size_cm: 5,
                building_cell_size_cm: 100,
            }));
        app.update();

        app.world_mut()
            .resource_mut::<Assets<ConfigAsset<TransformingTestConfig>>>()
            .insert(
                other_handle.id(),
                ConfigAsset::new(TransformingTestConfig {
                    base_cell_size_cm: 5,
                    building_cell_size_cm: 200,
                }),
            )
            .unwrap();
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<RuntimeTestConfig>(),
            &RuntimeTestConfig {
                base_cells_per_building_cell: 10
            }
        );
    }

    #[test]
    fn failed_reload_keeps_last_resolved_output() {
        let mut app = resolved_runtime_config_app();
        write_load_failure::<TransformingTestConfig>(&mut app, TEST_CONFIG_PATH);

        app.update();

        assert_eq!(
            app.world().resource::<RuntimeTestConfig>(),
            &RuntimeTestConfig {
                base_cells_per_building_cell: 10
            }
        );
        assert_eq!(
            app.world()
                .resource::<ConfigState<RuntimeTestConfig>>()
                .source(),
            Some(ConfigSource::Asset(TEST_CONFIG_PATH))
        );
    }

    #[test]
    fn failed_reload_transform_keeps_last_resolved_output() {
        let mut app = resolved_runtime_config_app();
        replace_target_config(
            &mut app,
            TransformingTestConfig {
                base_cell_size_cm: 0,
                building_cell_size_cm: 100,
            },
        );

        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<RuntimeTestConfig>(),
            &RuntimeTestConfig {
                base_cells_per_building_cell: 10
            }
        );
        assert_eq!(
            app.world()
                .resource::<ConfigState<RuntimeTestConfig>>()
                .source(),
            Some(ConfigSource::Asset(TEST_CONFIG_PATH))
        );
    }

    #[test]
    fn added_config_asset_recovers_from_default_fallback() {
        let mut app = runtime_config_app(MISSING_CONFIG_PATH);
        app.update();
        write_load_failure::<TransformingTestConfig>(&mut app, MISSING_CONFIG_PATH);
        app.update();

        replace_target_config(
            &mut app,
            TransformingTestConfig {
                base_cell_size_cm: 5,
                building_cell_size_cm: 100,
            },
        );
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<RuntimeTestConfig>(),
            &RuntimeTestConfig {
                base_cells_per_building_cell: 20
            }
        );
        assert_eq!(
            app.world()
                .resource::<ConfigState<RuntimeTestConfig>>()
                .source(),
            Some(ConfigSource::Asset(MISSING_CONFIG_PATH))
        );
    }

    fn runtime_config_app(path: ConfigPath) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<TransformingTestConfig>::new(path)
                .with_runtime_transformer()
                .fallback_with(|| TransformingTestConfig {
                    base_cell_size_cm: 5,
                    building_cell_size_cm: 50,
                }),
        ));
        app
    }

    fn resolved_runtime_config_app() -> App {
        let mut app = runtime_config_app(TEST_CONFIG_PATH);
        app.update();
        replace_target_config(
            &mut app,
            TransformingTestConfig {
                base_cell_size_cm: 5,
                building_cell_size_cm: 50,
            },
        );
        app.update();
        app
    }

    fn replace_target_config(app: &mut App, config: TransformingTestConfig) {
        let id = app
            .world()
            .resource::<ConfigHandle<TransformingTestConfig>>()
            .id();
        app.world_mut()
            .resource_mut::<Assets<ConfigAsset<TransformingTestConfig>>>()
            .insert(id, ConfigAsset::new(config))
            .unwrap();
    }

    fn write_load_failure<T>(app: &mut App, path: ConfigPath)
    where
        T: Send + Sync + 'static,
    {
        let handle = app.world().resource::<ConfigHandle<T>>();
        let id = handle.id();
        app.world_mut()
            .resource_mut::<Messages<AssetLoadFailedEvent<ConfigAsset<T>>>>()
            .write(AssetLoadFailedEvent {
                id,
                path: path.asset_path().into(),
                error: AssetLoadError::MissingAssetLoader {
                    asset_type_id: None,
                    asset_path: path.asset_path().to_owned(),
                },
            });
    }
}
