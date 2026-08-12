use crate::terrain::LandformSample;
use bevy::math::Vec3;

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
