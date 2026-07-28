use crate::foundation::config::ConfigHandle;
use bevy::asset::{Asset, Assets};
use bevy::prelude::{Commands, Res, ResMut, Resource};
use std::marker::PhantomData;
use tracing::error;

pub trait ConfigContext: Validate + ConfigRuntime {}

impl<T> ConfigContext for T where T: Validate + ConfigRuntime {}

pub trait ConfigRuntime: Asset {
    type Runtime: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn build_runtime(&self) -> Result<Self::Runtime, Self::Error>;
}

#[derive(Resource)]
pub struct ConfigRuntimeState<T: Asset> {
    applied: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Default for ConfigRuntimeState<T> {
    fn default() -> Self {
        Self {
            applied: false,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> ConfigRuntimeState<T> {
    pub const fn is_applied(&self) -> bool {
        self.applied
    }
}

pub fn apply_loaded_config_runtime<T>(
    mut commands: Commands,
    handle: Res<ConfigHandle<T>>,
    assets: Res<Assets<T>>,
    mut state: ResMut<ConfigRuntimeState<T>>,
) where
    T: ConfigContext,
{
    if state.applied {
        return;
    }

    let Some(config) = handle.get(&assets) else {
        return;
    };

    match config.build_runtime() {
        Ok(runtime) => {
            commands.insert_resource(runtime);
        }
        Err(error) => {
            error!(
                config_path = handle.path().asset_path(),
                error = %error,
                "failed to build runtime config"
            );
        }
    }

    state.applied = true;
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
