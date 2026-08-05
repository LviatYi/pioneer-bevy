use crate::foundation::config::{ConfigAsset, ConfigHandle, ConfigPath};
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

pub(crate) fn resolve_config_asset<T>(
    mut commands: Commands,
    handle: Res<ConfigHandle<T>>,
    assets: Res<Assets<ConfigAsset<T>>>,
    policy: Res<InitialLoadPolicy<T>>,
    mut state: ResMut<ConfigState<T>>,
    mut failures: MessageReader<AssetLoadFailedEvent<ConfigAsset<T>>>,
) where
    T: Clone + Resource,
{
    if state.is_resolved() {
        return;
    }

    for failure in failures.read() {
        if failure.id == handle.id() {
            let (config, source) = policy.handle_load_failure(handle.path(), failure);
            commands.insert_resource(config);
            state.status = ConfigStatus::Resolved { source };
            return;
        }
    }

    let Some(config) = handle.get(&assets) else {
        return;
    };

    let source = ConfigSource::Asset(handle.path());
    commands.insert_resource(config.clone());
    state.status = ConfigStatus::Resolved { source };
}

pub(crate) fn transform_config_to_runtime_resource<T>(
    mut commands: Commands,
    config: Option<Res<T>>,
    config_state: Res<ConfigState<T>>,
    mut state: ResMut<ConfigState<T::Runtime>>,
) where
    T: TransformToRuntimeResourceConfig + Resource,
{
    if state.is_resolved() {
        return;
    }

    let Some(config) = config else {
        return;
    };

    let source = config_state
        .source()
        .expect("config resource exists before config state is resolved");
    let runtime = config.transform_to_runtime().unwrap_or_else(|error| {
        panic!(
            "config `{}` failed to transform into a runtime resource: {error}",
            source.path().asset_path()
        )
    });

    commands.insert_resource(runtime);
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
