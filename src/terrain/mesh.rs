use crate::terrain::{TERRAIN_SAMPLE_SPACING_METERS, TerrainChunkSamples};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::{Mesh, Vec3};
use mcubes::{MarchingCubes, MeshSide};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerrainMeshError {
    #[error("terrain chunk sample dimensions are invalid: {0}")]
    InvalidSamples(#[from] std::io::Error),
    #[error("terrain chunk mesh has too many vertices for u32 indices")]
    TooManyVertices,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct TerrainMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl TerrainMeshData {
    pub(crate) fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub(crate) fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

pub(crate) fn extract_terrain_mesh(
    samples: &TerrainChunkSamples,
) -> Result<TerrainMeshData, TerrainMeshError> {
    let dimensions = samples.sample_dimensions();
    let cells = samples.cells();
    let marching_cubes = MarchingCubes::new(
        (
            dimensions.x as usize,
            dimensions.y as usize,
            dimensions.z as usize,
        ),
        (cells.x as f32, cells.y as f32, cells.z as f32),
        (cells.x as f32, cells.y as f32, cells.z as f32),
        Default::default(),
        samples.values().to_vec(),
        0.0,
    )?;
    let extracted = marching_cubes.generate(MeshSide::OutsideOnly);

    let mut positions = Vec::with_capacity(extracted.vertices.len());
    let mut normals = Vec::with_capacity(extracted.vertices.len());
    for vertex in extracted.vertices {
        positions.push([
            vertex.posit.x * TERRAIN_SAMPLE_SPACING_METERS,
            vertex.posit.y * TERRAIN_SAMPLE_SPACING_METERS,
            vertex.posit.z * TERRAIN_SAMPLE_SPACING_METERS,
        ]);

        // mcubes emits normals along the negative scalar-field gradient.
        // Pioneer's SDF gradient points from negative soil to positive air.
        normals.push([-vertex.normal.x, -vertex.normal.y, -vertex.normal.z]);
    }

    let mut indices = extracted
        .indices
        .into_iter()
        .map(|index| u32::try_from(index).map_err(|_| TerrainMeshError::TooManyVertices))
        .collect::<Result<Vec<_>, _>>()?;
    align_triangle_winding_with_normals(&positions, &normals, &mut indices);

    Ok(TerrainMeshData {
        positions,
        normals,
        indices,
    })
}

fn align_triangle_winding_with_normals(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    indices: &mut [u32],
) {
    for triangle in indices.chunks_exact_mut(3) {
        let a = Vec3::from_array(positions[triangle[0] as usize]);
        let b = Vec3::from_array(positions[triangle[1] as usize]);
        let c = Vec3::from_array(positions[triangle[2] as usize]);
        let face_normal = (b - a).cross(c - a);
        let vertex_normal = Vec3::from_array(normals[triangle[0] as usize]);

        if face_normal.dot(vertex_normal) < 0.0 {
            triangle.swap(1, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::landform_algorithm::flat::FlatLandform;
    use crate::terrain::{
        LandformGenerator, LandformSample, TERRAIN_CHUNK_CELLS, TerrainChunkCoord,
    };
    use bevy::prelude::UVec3;

    fn flat_chunk() -> TerrainChunkSamples {
        TerrainChunkSamples::sample(
            TerrainChunkCoord::new(0, -1, 0),
            Vec3::ZERO,
            UVec3::splat(TERRAIN_CHUNK_CELLS),
            &LandformGenerator::new(FlatLandform::default()),
        )
    }

    #[test]
    fn extracts_flat_surface_with_air_facing_normals_and_winding() {
        let mesh = extract_terrain_mesh(&flat_chunk()).unwrap();

        assert!(!mesh.is_empty());
        assert!(mesh.positions.iter().all(|position| position[1] == 16.0));
        assert!(mesh.normals.iter().all(|normal| normal[1] > 0.99));
        for triangle in mesh.indices.chunks_exact(3) {
            let a = Vec3::from_array(mesh.positions[triangle[0] as usize]);
            let b = Vec3::from_array(mesh.positions[triangle[1] as usize]);
            let c = Vec3::from_array(mesh.positions[triangle[2] as usize]);
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }

    #[test]
    fn extracted_vertices_follow_a_non_axis_aligned_field() {
        struct SlopedLandform;

        impl LandformSample for SlopedLandform {
            fn sample(&self, position: Vec3) -> f32 {
                position.x + position.y * 2.0 + position.z * 3.0 - 5.0
            }
        }

        let samples = TerrainChunkSamples::sample(
            TerrainChunkCoord::new(0, 0, 0),
            Vec3::ZERO,
            UVec3::splat(4),
            &SlopedLandform,
        );
        let mesh = extract_terrain_mesh(&samples).unwrap();
        let expected_normal = Vec3::new(1.0, 2.0, 3.0).normalize();

        assert!(!mesh.is_empty());
        for (position, normal) in mesh.positions.iter().zip(&mesh.normals) {
            let position = Vec3::from_array(*position);
            assert!(SlopedLandform.sample(position).abs() < 0.0001);
            assert!(Vec3::from_array(*normal).dot(expected_normal) > 0.99);
        }
    }
}
