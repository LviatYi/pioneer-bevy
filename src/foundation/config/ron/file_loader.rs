use crate::foundation::config::{
    ConfigLoadError, Validate, load_config_file, ron::RonConfigFormatError,
    ron::RonConfigFormatLoader,
};
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn load_ron_config<T>(
    path: impl AsRef<Path>,
) -> Result<T, ConfigLoadError<RonConfigFormatError, T::Error>>
where
    T: DeserializeOwned + Validate + 'static,
{
    load_config_file::<T, RonConfigFormatLoader<T>>(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, serde::Deserialize)]
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
    fn reports_read_errors() {
        let path = unique_test_path("missing.ron");

        let error = load_ron_config::<TestConfig>(&path).unwrap_err();

        assert!(matches!(error, ConfigLoadError::Read { .. }));
    }

    #[test]
    fn reports_ron_parse_errors() {
        let path = write_test_config("parse-error.ron", "not ron");

        let error = load_ron_config::<TestConfig>(&path).unwrap_err();

        assert!(matches!(error, ConfigLoadError::Format { .. }));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn reports_validation_errors() {
        let path = write_test_config("validation-error.ron", "(value: 0)");

        let error = load_ron_config::<TestConfig>(&path).unwrap_err();

        assert!(matches!(error, ConfigLoadError::Validation { .. }));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn loads_valid_ron_configs() {
        let path = write_test_config("valid.ron", "(value: 7)");

        let config = load_ron_config::<TestConfig>(&path).unwrap();

        assert_eq!(config.value, 7);
        std::fs::remove_file(path).ok();
    }

    fn write_test_config(file_name: &str, source: &str) -> PathBuf {
        let path = unique_test_path(file_name);
        std::fs::write(&path, source).unwrap();
        path
    }

    fn unique_test_path(file_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pioneer-bevy-config-test-{}-{file_name}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
