use crate::foundation::config::{ConfigFormatLoader, ConfigLoadError, Validate};
use std::fs;
use std::path::Path;

pub fn load_config_file<T, F>(
    path: impl AsRef<Path>,
) -> Result<T, ConfigLoadError<F::Error, T::Error>>
where
    T: Validate,
    F: ConfigFormatLoader<T>,
{
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| ConfigLoadError::Read {
        path: path.to_owned(),
        source,
    })?;
    let config = F::default()
        .load(&bytes)
        .map_err(|source| ConfigLoadError::Format {
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
