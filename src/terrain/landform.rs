use bevy::prelude::{Resource, Vec3};
use std::sync::Arc;

/// A continuous scalar field describing the boundary between soil and air.
///
/// Negative values are soil, positive values are air, and zero is the terrain
/// surface.
pub trait LandformSample: Send + Sync + 'static {
    fn sample(&self, position: Vec3) -> f32;
}

/// The active, replaceable base landform generator.
#[derive(Clone, Resource)]
pub struct LandformGenerator {
    generator: Arc<dyn LandformSample>,
    revision: u64,
}

impl LandformGenerator {
    pub fn new(generator: impl LandformSample) -> Self {
        Self {
            generator: Arc::new(generator),
            revision: 0,
        }
    }

    pub fn sample(&self, position: Vec3) -> f32 {
        self.generator.sample(position)
    }

    pub fn replace(&mut self, generator: impl LandformSample) {
        self.generator = Arc::new(generator);
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }
}

impl LandformSample for LandformGenerator {
    fn sample(&self, position: Vec3) -> f32 {
        self.generator.sample(position)
    }
}

impl Default for LandformGenerator {
    fn default() -> Self {
        Self::new(FlatLandform::default())
    }
}

/// MVP landform: an infinite horizontal surface.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlatLandform {
    pub surface_height: f32,
}

impl LandformSample for FlatLandform {
    fn sample(&self, position: Vec3) -> f32 {
        position.y - self.surface_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_landform_uses_signed_distance_convention() {
        let field = FlatLandform {
            surface_height: 2.0,
        };

        assert!(field.sample(Vec3::new(0.0, 1.0, 0.0)) < 0.0);
        assert_eq!(field.sample(Vec3::new(5.0, 2.0, -3.0)), 0.0);
        assert!(field.sample(Vec3::new(0.0, 3.0, 0.0)) > 0.0);
    }

    #[test]
    fn replacing_generator_changes_revision() {
        let mut generator = LandformGenerator::default();
        let revision = generator.revision();

        generator.replace(FlatLandform {
            surface_height: 4.0,
        });

        assert_ne!(generator.revision(), revision);
        assert_eq!(generator.sample(Vec3::Y * 4.0), 0.0);
    }
}
