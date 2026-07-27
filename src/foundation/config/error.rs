use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigLoadError<ValidationError>
where
    ValidationError: std::error::Error + Send + Sync + 'static,
{
    #[error("failed to read config `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config `{path}` as RON")]
    ParseRon {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("config `{path}` failed validation")]
    Validation {
        path: PathBuf,
        #[source]
        source: ValidationError,
    },
}
