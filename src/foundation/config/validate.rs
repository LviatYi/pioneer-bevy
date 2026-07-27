pub trait Validate: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn validate(&self) -> Result<(), Self::Error>;

    fn validated(self) -> Result<Self, Self::Error> {
        self.validate()?;
        Ok(self)
    }
}
