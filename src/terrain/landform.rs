use crate::terrain::landform_algorithm::flat::FlatLandform;
use bevy::prelude::{Resource, Vec3};
use std::sync::Arc;

/// A continuous scalar field describing the boundary between soil and air.
///
/// Negative values are soil, positive values are air, and zero is the terrain
/// surface.
pub trait LandformSample: Send + Sync + 'static {
    fn sample(&self, position: Vec3) -> f32;
}

/// The active base landform generator.
#[derive(Clone, Resource)]
pub struct LandformGenerator {
    generator: Arc<dyn LandformSample>,
}

impl LandformGenerator {
    pub fn new(generator: impl LandformSample) -> Self {
        Self {
            generator: Arc::new(generator),
        }
    }

    pub fn sample(&self, position: Vec3) -> f32 {
        self.generator.sample(position)
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
