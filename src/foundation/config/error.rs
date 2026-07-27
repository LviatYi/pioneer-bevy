use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigLoadError<FormatError, ValidationError>
where
    FormatError: std::error::Error + Send + Sync + 'static,
    ValidationError: std::error::Error + Send + Sync + 'static,
{
    #[error("failed to read config `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to load config `{path}` from its storage format")]
    Format {
        path: PathBuf,
        #[source]
        source: FormatError,
    },
    #[error("config `{path}` failed validation")]
    Validation {
        path: PathBuf,
        #[source]
        source: ValidationError,
    },
}
