use crate::foundation::config::{TransformToRuntimeResourceConfig, ValidateConfig};
use crate::terrain::{LandformGenerator, LandformSample};
use bevy::math::Vec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Parameters for a gently rolling Perlin fBm plain.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct PlainsLandformConfig {
    /// Selects a deterministic noise field.
    pub seed: u32,
    /// Average surface elevation in world meters.
    pub base_height_meters: f32,
    /// Maximum intended displacement around the average height, in meters.
    pub amplitude_meters: f32,
    /// World-space wavelength of the first and broadest octave, in meters.
    pub wavelength_meters: f32,
    /// Number of noise layers. More layers add smaller details and sampling cost.
    pub octaves: u8,
    /// Frequency multiplier between successive octaves.
    pub lacunarity: f32,
    /// Amplitude multiplier between successive octaves.
    pub persistence: f32,
}

impl Default for PlainsLandformConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            base_height_meters: 0.0,
            amplitude_meters: 2.0,
            wavelength_meters: 64.0,
            octaves: 4,
            lacunarity: 2.0,
            persistence: 0.42,
        }
    }
}

impl ValidateConfig for PlainsLandformConfig {
    type Error = PlainsLandformConfigValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        validate_finite("base_height_meters", self.base_height_meters)?;

        if !self.amplitude_meters.is_finite() || self.amplitude_meters < 0.0 {
            return Err(PlainsLandformConfigValidationError::InvalidParameter {
                field: "amplitude_meters",
                requirement: "must be finite and greater than or equal to 0",
            });
        }
        if !self.wavelength_meters.is_finite() || self.wavelength_meters <= 0.0 {
            return Err(PlainsLandformConfigValidationError::InvalidParameter {
                field: "wavelength_meters",
                requirement: "must be finite and greater than 0",
            });
        }
        if !(1..=Fbm::<Perlin>::MAX_OCTAVES as u8).contains(&self.octaves) {
            return Err(PlainsLandformConfigValidationError::InvalidParameter {
                field: "octaves",
                requirement: "must be between 1 and 32",
            });
        }
        if !self.lacunarity.is_finite() || self.lacunarity <= 1.0 {
            return Err(PlainsLandformConfigValidationError::InvalidParameter {
                field: "lacunarity",
                requirement: "must be finite and greater than 1",
            });
        }
        if !self.persistence.is_finite() || self.persistence <= 0.0 || self.persistence >= 1.0 {
            return Err(PlainsLandformConfigValidationError::InvalidParameter {
                field: "persistence",
                requirement: "must be finite and between 0 and 1",
            });
        }

        Ok(())
    }
}

fn validate_finite(
    field: &'static str,
    value: f32,
) -> Result<(), PlainsLandformConfigValidationError> {
    if !value.is_finite() {
        return Err(PlainsLandformConfigValidationError::InvalidParameter {
            field,
            requirement: "must be finite",
        });
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum PlainsLandformConfigValidationError {
    #[error("plains landform config field `{field}` {requirement}")]
    InvalidParameter {
        field: &'static str,
        requirement: &'static str,
    },
}

impl TransformToRuntimeResourceConfig for PlainsLandformConfig {
    type Runtime = LandformGenerator;
    type Error = PlainsLandformConfigValidationError;

    fn transform_to_runtime(&self) -> Result<Self::Runtime, Self::Error> {
        self.validate()?;
        Ok(LandformGenerator::new(PlainsLandform::from_config(*self)))
    }
}

/// A height-field plain backed by noise-rs' Perlin fBm implementation.
#[derive(Clone, Debug)]
pub struct PlainsLandform {
    noise: Fbm<Perlin>,
    base_height_meters: f32,
    amplitude_meters: f32,
}

impl PlainsLandform {
    pub fn from_config(config: PlainsLandformConfig) -> Self {
        let noise = Fbm::<Perlin>::new(config.seed)
            .set_frequency(1.0 / f64::from(config.wavelength_meters))
            .set_octaves(usize::from(config.octaves))
            .set_lacunarity(f64::from(config.lacunarity))
            .set_persistence(f64::from(config.persistence));

        Self {
            noise,
            base_height_meters: config.base_height_meters,
            amplitude_meters: config.amplitude_meters,
        }
    }

    pub fn surface_height_at(&self, x: f32, z: f32) -> f32 {
        let noise = self.noise.get([f64::from(x), f64::from(z)]) as f32;
        self.base_height_meters + self.amplitude_meters * noise
    }
}

impl LandformSample for PlainsLandform {
    fn sample(&self, position: Vec3) -> f32 {
        position.y - self.surface_height_at(position.x, position.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_isosurface_follows_the_generated_height() {
        let plains = PlainsLandform::from_config(PlainsLandformConfig::default());
        let x = 11.25;
        let z = -7.5;
        let surface_height = plains.surface_height_at(x, z);

        assert_eq!(plains.sample(Vec3::new(x, surface_height, z)), 0.0);
        assert!(plains.sample(Vec3::new(x, surface_height + 1.0, z)) > 0.0);
        assert!(plains.sample(Vec3::new(x, surface_height - 1.0, z)) < 0.0);
    }

    #[test]
    fn rejects_parameters_that_do_not_form_a_decaying_fbm() {
        let invalid = PlainsLandformConfig {
            persistence: 1.0,
            ..Default::default()
        };

        assert!(invalid.validate().is_err());
    }
}
