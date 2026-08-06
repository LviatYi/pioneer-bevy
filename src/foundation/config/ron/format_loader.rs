use super::super::asset::{
    ConfigFormatLoader, ConfigPlugin, NoConfigValidation, RawResourceOutput,
};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use thiserror::Error;

/// Abbreviation for `ConfigPlugin<T, RonConfigFormatLoader<T>, V, O>`.
pub type RonConfigPlugin<T, V = NoConfigValidation, O = RawResourceOutput> =
    ConfigPlugin<T, RonConfigFormatLoader<T>, V, O>;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestConfig {
        _value: u32,
    }

    #[test]
    fn reports_parse_errors() {
        let error = RonConfigFormatLoader::<TestConfig>::default()
            .load(b"not ron")
            .unwrap_err();

        assert!(matches!(error, RonConfigFormatError::Parse(_)));
    }
}
