use super::asset::{ConfigAsset, ConfigHandle, ConfigOutput};
use super::path::ConfigPath;
use bevy::asset::{AssetLoadFailedEvent, Assets};
use bevy::prelude::{Commands, MessageReader, Res, ResMut, Resource};
use std::marker::PhantomData;
use tracing::warn;

pub trait TransformToRuntimeResourceConfig {
    type Runtime: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error>;
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigStatus {
    Loading,
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
            status: ConfigStatus::Loading,
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
            ConfigStatus::Loading => None,
            ConfigStatus::Resolved { source } => Some(source),
        }
    }
}

pub(super) fn resolve_config_output<T, O>(
    mut commands: Commands,
    handle: Res<ConfigHandle<T>>,
    assets: Res<Assets<ConfigAsset<T>>>,
    policy: Res<InitialLoadPolicy<T>>,
    mut state: ResMut<ConfigState<O::Output>>,
    mut failures: MessageReader<AssetLoadFailedEvent<ConfigAsset<T>>>,
) where
    T: Send + Sync + 'static,
    O: ConfigOutput<T>,
{
    if state.is_resolved() {
        return;
    }

    for failure in failures.read() {
        if failure.id == handle.id() {
            let (config, source) = policy.handle_load_failure(handle.path(), failure);
            commit_config_output::<T, O>(&mut commands, &mut state, &config, source);
            return;
        }
    }

    let Some(config) = handle.get(&assets) else {
        return;
    };

    let source = ConfigSource::Asset(handle.path());
    commit_config_output::<T, O>(&mut commands, &mut state, config, source);
}

fn commit_config_output<T, O>(
    commands: &mut Commands,
    state: &mut ConfigState<O::Output>,
    config: &T,
    source: ConfigSource,
) where
    O: ConfigOutput<T>,
{
    let output = O::produce(config).unwrap_or_else(|error| {
        panic!(
            "config `{}` failed to produce its resource output: {error}",
            source.path().asset_path()
        )
    });

    commands.insert_resource(output);
    state.status = ConfigStatus::Resolved { source };
}

impl ConfigSource {
    pub const fn path(self) -> ConfigPath {
        match self {
            Self::Asset(path) | Self::DefaultFallback(path) => path,
        }
    }
}

pub trait ValidateConfig: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self) -> Result<(), Self::Error>;

    fn validated(self) -> Result<Self, Self::Error> {
        self.validate()?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::asset::{ConfigAsset, ConfigHandle};
    use super::super::ron::RonConfigPlugin;
    use super::*;
    use bevy::asset::{AssetLoadError, AssetLoadFailedEvent, AssetPlugin};
    use bevy::ecs::message::Messages;
    use bevy::prelude::*;
    use serde::Deserialize;
    use std::convert::Infallible;

    const TEST_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");
    const MISSING_CONFIG_PATH: ConfigPath = ConfigPath::new("config/missing-config.ron");

    #[derive(Debug, Default, Deserialize)]
    struct TransformingTestConfig {
        base_cell_size_cm: u32,
        building_cell_size_cm: u32,
    }

    impl TransformToRuntimeResourceConfig for TransformingTestConfig {
        type Runtime = RuntimeTestConfig;
        type Error = Infallible;

        fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error> {
            Ok(RuntimeTestConfig {
                base_cells_per_building_cell: self.building_cell_size_cm / self.base_cell_size_cm,
            })
        }
    }

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

        write_load_failure::<TransformingTestConfig>(&mut app);
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

        write_load_failure::<RawTestConfig>(&mut app);
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

    fn write_load_failure<T>(app: &mut App)
    where
        T: Send + Sync + 'static,
    {
        let handle = app.world().resource::<ConfigHandle<T>>();
        let id = handle.id();
        app.world_mut()
            .resource_mut::<Messages<AssetLoadFailedEvent<ConfigAsset<T>>>>()
            .write(AssetLoadFailedEvent {
                id,
                path: MISSING_CONFIG_PATH.asset_path().into(),
                error: AssetLoadError::MissingAssetLoader {
                    asset_type_id: None,
                    asset_path: MISSING_CONFIG_PATH.asset_path().to_owned(),
                },
            });
    }
}
