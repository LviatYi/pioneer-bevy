use crate::foundation::config::{
    ConfigFormatLoader, ConfigPlugin, NoConfigRuntime, NoConfigValidation,
};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use thiserror::Error;

/// Abbreviation for `ConfigPlugin<T, RonConfigFormatLoader<T>, V, R>`.
pub type RonConfigPlugin<T, V = NoConfigValidation, R = NoConfigRuntime> =
    ConfigPlugin<T, RonConfigFormatLoader<T>, V, R>;

pub struct RonConfigFormatLoader<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RonConfigFormatLoader<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> ConfigFormatLoader<T> for RonConfigFormatLoader<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = RonConfigFormatError;

    fn load(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        ron::de::from_bytes::<T>(bytes).map_err(RonConfigFormatError::Parse)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

#[derive(Debug, Error)]
pub enum RonConfigFormatError {
    #[error("failed to parse config as RON")]
    Parse(#[source] ron::error::SpannedError),
}
