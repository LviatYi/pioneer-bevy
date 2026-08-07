use super::error::NoConfigValidationError;

pub trait ValidateConfig: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self) -> Result<(), Self::Error>;

    fn validated(self) -> Result<Self, Self::Error> {
        self.validate()?;
        Ok(self)
    }
}

pub trait ConfigValidation<T>: Default + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self, config: &T) -> Result<(), Self::Error>;
}

#[derive(Default)]
pub struct NoConfigValidation;

impl<T> ConfigValidation<T> for NoConfigValidation {
    type Error = NoConfigValidationError;

    fn validate(&self, _config: &T) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Default)]
#[doc(hidden)]
pub struct WithValidation;

impl<T> ConfigValidation<T> for WithValidation
where
    T: ValidateConfig,
{
    type Error = T::Error;

    fn validate(&self, config: &T) -> Result<(), Self::Error> {
        config.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt::{Display, Formatter};

    struct TestConfig {
        value: u32,
    }

    impl ValidateConfig for TestConfig {
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
    fn validation_reports_validation_errors() {
        let error = WithValidation
            .validate(&TestConfig { value: 0 })
            .unwrap_err();

        assert!(matches!(error, TestConfigValidationError));
    }

    #[test]
    fn validation_accepts_valid_configs() {
        let config = TestConfig { value: 7 };

        assert!(WithValidation.validate(&config).is_ok());
    }

    #[test]
    fn no_validation_accepts_configs_without_validation() {
        let config = TestConfig { value: 0 };

        assert!(NoConfigValidation.validate(&config).is_ok());
    }
}
