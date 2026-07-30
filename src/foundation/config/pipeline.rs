use crate::foundation::config::{
    ConfigFormatLoader, ConfigLoadError, NoConfigValidationError, Validate,
};

pub fn load_config_bytes<T, F>(
    bytes: &[u8],
    path: impl Into<String>,
) -> Result<T, ConfigLoadError<F::Error, NoConfigValidationError>>
where
    F: ConfigFormatLoader<T>,
{
    let path = path.into();
    F::default()
        .load(bytes)
        .map_err(|source| ConfigLoadError::Format { path, source })
}

pub fn load_validated_config_bytes<T, F>(
    bytes: &[u8],
    path: impl Into<String>,
) -> Result<T, ConfigLoadError<F::Error, T::Error>>
where
    T: Validate,
    F: ConfigFormatLoader<T>,
{
    let path = path.into();
    let config = F::default()
        .load(bytes)
        .map_err(|source| ConfigLoadError::Format {
            path: path.clone(),
            source,
        })?;
    config
        .validate()
        .map_err(|source| ConfigLoadError::Validation { path, source })?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::config::ron::RonConfigFormatLoader;
    use serde::Deserialize;
    use std::error::Error;
    use std::fmt::{Display, Formatter};

    #[derive(Debug, Deserialize)]
    struct TestConfig {
        value: u32,
    }

    impl Validate for TestConfig {
        type Error = TestConfigValidationError;

        fn validate(&self) -> Result<(), Self::Error> {
            if self.value == 0 {
                return Err(TestConfigValidationError);
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    struct TestConfigValidationError;

    impl Display for TestConfigValidationError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("value must be greater than 0")
        }
    }

    impl Error for TestConfigValidationError {}

    #[test]
    fn reports_format_errors() {
        let error = load_validated_config_bytes::<TestConfig, RonConfigFormatLoader<TestConfig>>(
            b"not ron", "test",
        )
        .unwrap_err();

        assert!(matches!(error, ConfigLoadError::Format { .. }));
    }

    #[test]
    fn reports_validation_errors() {
        let error = load_validated_config_bytes::<TestConfig, RonConfigFormatLoader<TestConfig>>(
            b"(value: 0)",
            "test",
        )
        .unwrap_err();

        assert!(matches!(error, ConfigLoadError::Validation { .. }));
    }

    #[test]
    fn loads_valid_configs() {
        let config = load_validated_config_bytes::<TestConfig, RonConfigFormatLoader<TestConfig>>(
            b"(value: 7)",
            "test",
        )
        .unwrap();

        assert_eq!(config.value, 7);
    }

    #[test]
    fn loads_configs_without_validation_by_default() {
        let config = load_config_bytes::<TestConfig, RonConfigFormatLoader<TestConfig>>(
            b"(value: 0)",
            "test",
        )
        .unwrap();

        assert_eq!(config.value, 0);
    }
}
