use crate::foundation::config::{ConfigHandle, ConfigPath};
use bevy::asset::{Asset, AssetLoadFailedEvent, Assets};
use bevy::prelude::{Commands, MessageReader, Res, ResMut, Resource};
use std::marker::PhantomData;
use tracing::warn;

pub trait ConfigContext: Validate + ConfigRuntime {}

impl<T> ConfigContext for T where T: Validate + ConfigRuntime {}

pub trait ConfigRuntime: Asset {
    type Runtime: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn build_runtime(&self) -> Result<Self::Runtime, Self::Error>;
}

#[derive(Resource)]
pub enum ConfigLoadPolicy<T> {
    Required,
    FallbackToDefault { default: fn() -> T },
}

impl<T> Clone for ConfigLoadPolicy<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ConfigLoadPolicy<T> {}

impl<T> ConfigLoadPolicy<T> {
    pub const fn required() -> Self {
        Self::Required
    }

    pub const fn fallback_with(default: fn() -> T) -> Self {
        Self::FallbackToDefault { default }
    }
}

impl<T> ConfigLoadPolicy<T>
where
    T: Default,
{
    pub fn fallback_to_default() -> Self {
        Self::FallbackToDefault {
            default: T::default,
        }
    }
}

impl<T> ConfigLoadPolicy<T>
where
    T: ConfigRuntime,
{
    fn handle_load_failure(
        self,
        handle: &ConfigHandle<T>,
        failure: &AssetLoadFailedEvent<T>,
    ) -> (T::Runtime, ConfigSource) {
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

                (
                    build_default_runtime(handle, default),
                    ConfigSource::DefaultFallback(handle.path()),
                )
            }
        }
    }

    fn handle_runtime_failure(
        self,
        handle: &ConfigHandle<T>,
        error: <T as ConfigRuntime>::Error,
    ) -> (T::Runtime, ConfigSource) {
        match self {
            Self::Required => {
                panic!(
                    "required config `{}` failed to build runtime: {error}",
                    handle.path().asset_path()
                );
            }
            Self::FallbackToDefault { default } => {
                warn!(
                    config_path = handle.path().asset_path(),
                    error = %error,
                    "config failed to build runtime; using default fallback"
                );

                (
                    build_default_runtime(handle, default),
                    ConfigSource::DefaultFallback(handle.path()),
                )
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
pub enum ConfigRuntimeStatus {
    Loading,
    Applied { source: ConfigSource },
    Failed { message: String },
}

#[derive(Resource)]
pub struct ConfigRuntimeState<T: Asset> {
    status: ConfigRuntimeStatus,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Default for ConfigRuntimeState<T> {
    fn default() -> Self {
        Self {
            status: ConfigRuntimeStatus::Loading,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> ConfigRuntimeState<T> {
    pub const fn is_applied(&self) -> bool {
        matches!(self.status, ConfigRuntimeStatus::Applied { .. })
    }

    pub const fn status(&self) -> &ConfigRuntimeStatus {
        &self.status
    }
}

pub fn apply_config_runtime<T>(
    mut commands: Commands,
    handle: Res<ConfigHandle<T>>,
    assets: Res<Assets<T>>,
    policy: Res<ConfigLoadPolicy<T>>,
    mut state: ResMut<ConfigRuntimeState<T>>,
    mut failures: MessageReader<AssetLoadFailedEvent<T>>,
) where
    T: ConfigContext,
{
    if state.is_applied() {
        return;
    }

    for failure in failures.read() {
        if failure.id == handle.id() {
            let (runtime, source) = policy.handle_load_failure(&handle, failure);
            commands.insert_resource(runtime);
            state.status = ConfigRuntimeStatus::Applied { source };
            return;
        }
    }

    let Some(config) = handle.get(&assets) else {
        return;
    };

    let runtime = match config.build_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            let (runtime, source) = policy.handle_runtime_failure(&handle, error);
            commands.insert_resource(runtime);
            state.status = ConfigRuntimeStatus::Applied { source };
            return;
        }
    };

    commands.insert_resource(runtime);
    state.status = ConfigRuntimeStatus::Applied {
        source: ConfigSource::Asset(handle.path()),
    };
}

fn build_default_runtime<T>(handle: &ConfigHandle<T>, default: fn() -> T) -> T::Runtime
where
    T: ConfigRuntime,
{
    default().build_runtime().unwrap_or_else(|error| {
        panic!(
            "default fallback for config `{}` failed to build runtime: {error}",
            handle.path().asset_path()
        )
    })
}

pub trait Validate: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validated(self) -> Result<Self, Self::Error> {
        self.validate()?;
        Ok(self)
    }
}
