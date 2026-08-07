use bevy::prelude::Resource;
use std::convert::Infallible;

pub trait TransformToRuntimeResourceConfig {
    type Runtime: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error>;
}

#[derive(Default)]
#[doc(hidden)]
pub struct RawResourceOutput;

#[derive(Default)]
#[doc(hidden)]
pub struct RuntimeResourceOutput;

pub(super) trait ConfigOutput<T>: Send + Sync + 'static {
    type Output: Resource;
    type Error: std::error::Error + Send + Sync + 'static;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error>;
}

impl<T> ConfigOutput<T> for RawResourceOutput
where
    T: Clone + Resource,
{
    type Output = T;
    type Error = Infallible;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error> {
        Ok(raw.clone())
    }
}

impl<T> ConfigOutput<T> for RuntimeResourceOutput
where
    T: TransformToRuntimeResourceConfig,
{
    type Output = T::Runtime;
    type Error = T::Error;

    fn produce(raw: &T) -> Result<Self::Output, Self::Error> {
        raw.transform_to_runtime()
    }
}
