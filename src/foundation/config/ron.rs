use crate::foundation::config::{ConfigLoadError, Validate};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

pub fn load_ron_config<T>(path: impl AsRef<Path>) -> Result<T, ConfigLoadError<T::Error>>
where
    T: DeserializeOwned + Validate,
{
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| ConfigLoadError::Read {
        path: path.to_owned(),
        source,
    })?;
    let config = ron::from_str::<T>(&source).map_err(|source| ConfigLoadError::ParseRon {
        path: path.to_owned(),
        source,
    })?;

    config
        .validate()
        .map_err(|source| ConfigLoadError::Validation {
            path: path.to_owned(),
            source,
        })?;

    Ok(config)
}
