use crate::foundation::config::{TransformToRuntimeResourceConfig, ValidateConfig};
use crate::terrain::TerrainBounds;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Resource, Serialize)]
#[serde(default)]
pub struct TerrainConfig {
    pub side_length_meters: u32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            side_length_meters: 32,
        }
    }
}

impl ValidateConfig for TerrainConfig {
    type Error = TerrainConfigValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.side_length_meters == 0 {
            return Err(TerrainConfigValidationError::SideLengthMustBePositive);
        }

        Ok(())
    }
}

impl TransformToRuntimeResourceConfig for TerrainConfig {
    type Runtime = TerrainBounds;
    type Error = TerrainConfigValidationError;

    fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error> {
        TerrainBounds::from_config(*self)
    }
}

#[derive(Debug, Error)]
pub enum TerrainConfigValidationError {
    #[error("terrain side length in meters must be greater than 0")]
    SideLengthMustBePositive,
}
