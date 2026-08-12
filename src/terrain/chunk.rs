use crate::terrain::{LandformSample, TerrainConfig};
use bevy::platform::collections::HashMap;
use bevy::prelude::{IVec3, Resource, UVec3, Vec3};

pub const TERRAIN_CHUNK_CELLS: u32 = 16;
pub const TERRAIN_SAMPLE_SPACING_METERS: f32 = 1.0;

/// Integer coordinate of a terrain mesh chunk.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TerrainChunkCoord(pub IVec3);

impl TerrainChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn origin(self, terrain_origin: Vec3) -> Vec3 {
        terrain_origin
            + self.0.as_vec3() * (TERRAIN_CHUNK_CELLS as f32 * TERRAIN_SAMPLE_SPACING_METERS)
    }
}

/// Cached samples for one regular three-dimensional grid.
#[derive(Clone, Debug)]
pub struct TerrainChunkSamples {
    coord: TerrainChunkCoord,
    origin: Vec3,
    cells: UVec3,
    values: Vec<f32>,
}

/// Sparse cache of sampled terrain chunks.
#[derive(Default, Resource)]
pub struct TerrainChunkCache {
    chunks: HashMap<TerrainChunkCoord, TerrainChunkSamples>,
}

impl TerrainChunkCache {
    pub fn get(&self, coord: TerrainChunkCoord) -> Option<&TerrainChunkSamples> {
        self.chunks.get(&coord)
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.chunks.clear();
    }

    pub(crate) fn insert(&mut self, samples: TerrainChunkSamples) {
        self.chunks.insert(samples.coord(), samples);
    }
}

impl TerrainChunkSamples {
    pub fn sample<F: LandformSample + ?Sized>(
        coord: TerrainChunkCoord,
        terrain_origin: Vec3,
        cells: UVec3,
        field: &F,
    ) -> Self {
        let origin = coord.origin(terrain_origin);
        let sample_dimensions = cells + UVec3::ONE;
        let mut values = Vec::with_capacity(sample_dimensions.element_product() as usize);

        // X is the fastest-changing axis. Positions are derived directly from
        // integer sample coordinates, never by repeated floating-point addition.
        for z in 0..sample_dimensions.z {
            for y in 0..sample_dimensions.y {
                for x in 0..sample_dimensions.x {
                    let local_sample = UVec3::new(x, y, z).as_vec3();
                    let position = origin + local_sample * TERRAIN_SAMPLE_SPACING_METERS;
                    values.push(field.sample(position));
                }
            }
        }

        Self {
            coord,
            origin,
            cells,
            values,
        }
    }

    pub const fn coord(&self) -> TerrainChunkCoord {
        self.coord
    }

    pub const fn origin(&self) -> Vec3 {
        self.origin
    }

    pub const fn cells(&self) -> UVec3 {
        self.cells
    }

    pub fn sample_dimensions(&self) -> UVec3 {
        self.cells + UVec3::ONE
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn value(&self, local_sample: UVec3) -> Option<f32> {
        let dimensions = self.sample_dimensions();
        if local_sample.cmpge(dimensions).any() {
            return None;
        }

        let index = local_sample.x
            + local_sample.y * dimensions.x
            + local_sample.z * dimensions.x * dimensions.y;
        self.values.get(index as usize).copied()
    }
}

/// Chunks needed by the MVP terrain renderer.
///
/// Two Y layers around zero provide an initial generation volume for both
/// below-ground structures and positive-height landforms.
pub(crate) fn mvp_surface_chunks(config: TerrainConfig) -> Vec<(TerrainChunkCoord, UVec3)> {
    let side = config.side_length_meters;
    let chunks_per_side = side.div_ceil(TERRAIN_CHUNK_CELLS);
    let mut chunks = Vec::with_capacity((chunks_per_side * chunks_per_side * 2) as usize);

    for chunk_y in -1..=0 {
        for chunk_z in 0..chunks_per_side {
            for chunk_x in 0..chunks_per_side {
                let remaining_x = side - chunk_x * TERRAIN_CHUNK_CELLS;
                let remaining_z = side - chunk_z * TERRAIN_CHUNK_CELLS;
                chunks.push((
                    TerrainChunkCoord::new(chunk_x as i32, chunk_y, chunk_z as i32),
                    UVec3::new(
                        remaining_x.min(TERRAIN_CHUNK_CELLS),
                        TERRAIN_CHUNK_CELLS,
                        remaining_z.min(TERRAIN_CHUNK_CELLS),
                    ),
                ));
            }
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::landform_algorithm::flat::FlatLandform;
    use crate::terrain::{LandformGenerator, TerrainConfig};

    #[test]
    fn chunk_has_one_more_sample_than_cell_on_each_axis() {
        let field = LandformGenerator::new(FlatLandform::default());
        let samples = TerrainChunkSamples::sample(
            TerrainChunkCoord::new(0, -1, 0),
            Vec3::ZERO,
            UVec3::splat(2),
            &field,
        );

        assert_eq!(samples.sample_dimensions(), UVec3::splat(3));
        assert_eq!(samples.values().len(), 27);
    }

    #[test]
    fn adjacent_chunks_share_identical_boundary_samples() {
        let field = LandformGenerator::new(FlatLandform {
            surface_height: 0.25,
        });
        let terrain_origin = Vec3::new(-16.0, 0.0, -16.0);
        let left = TerrainChunkSamples::sample(
            TerrainChunkCoord::new(0, -1, 0),
            terrain_origin,
            UVec3::splat(TERRAIN_CHUNK_CELLS),
            &field,
        );
        let right = TerrainChunkSamples::sample(
            TerrainChunkCoord::new(1, -1, 0),
            terrain_origin,
            UVec3::splat(TERRAIN_CHUNK_CELLS),
            &field,
        );

        for z in 0..=TERRAIN_CHUNK_CELLS {
            for y in 0..=TERRAIN_CHUNK_CELLS {
                assert_eq!(
                    left.value(UVec3::new(TERRAIN_CHUNK_CELLS, y, z)),
                    right.value(UVec3::new(0, y, z)),
                );
            }
        }
    }

    #[test]
    fn edge_chunks_are_trimmed_to_terrain_bounds() {
        let config = TerrainConfig {
            side_length_meters: 17,
        };
        let chunks = mvp_surface_chunks(config);

        assert_eq!(chunks.len(), 8);
        assert_eq!(chunks[0].1, UVec3::new(16, 16, 16));
        assert_eq!(chunks[3].1, UVec3::new(1, 16, 1));
        assert_eq!(chunks[7].0, TerrainChunkCoord::new(1, 0, 1));
    }
}
