use crate::foundation::config::ValidateConfig;
use crate::grid::{GridCoord, GridSet};
use bevy::prelude::{Resource, Vec3};
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

impl TerrainConfig {
    pub fn contains_surface_cell(self, coord: GridCoord, grids: GridSet) -> bool {
        self.contains_footprint(coord, 1, 1, grids)
    }

    pub fn contains_world_xz(self, position: Vec3) -> bool {
        let world_min = self.world_min();
        let world_max = self.world_max();

        position.x >= world_min.x
            && position.x < world_max.x
            && position.z >= world_min.z
            && position.z < world_max.z
    }

    pub fn contains_footprint(
        self,
        origin: GridCoord,
        width_building_cells: u32,
        depth_building_cells: u32,
        grids: GridSet,
    ) -> bool {
        if origin.y != 0 || width_building_cells == 0 || depth_building_cells == 0 {
            return false;
        }

        let Some(end_x) = origin.x.checked_add(i64::from(width_building_cells)) else {
            return false;
        };
        let Some(end_z) = origin.z.checked_add(i64::from(depth_building_cells)) else {
            return false;
        };

        let building_grid = grids.building();
        let footprint_min = building_grid.cell_origin_world(origin);
        let footprint_max = building_grid.cell_origin_world(GridCoord::new(end_x, 0, end_z));
        let world_min = self.world_min();
        let world_max = self.world_max();

        footprint_min.x >= world_min.x
            && footprint_min.z >= world_min.z
            && footprint_max.x <= world_max.x
            && footprint_max.z <= world_max.z
    }

    pub fn world_min(self) -> Vec3 {
        let half_side = self.side_length_meters as f32 * 0.5;
        Vec3::new(-half_side, 0.0, -half_side)
    }

    pub fn world_max(self) -> Vec3 {
        let half_side = self.side_length_meters as f32 * 0.5;
        Vec3::new(half_side, 0.0, half_side)
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

#[derive(Debug, Error)]
pub enum TerrainConfigValidationError {
    #[error("terrain side length in meters must be greater than 0")]
    SideLengthMustBePositive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridConfig;

    #[test]
    fn creates_centered_metric_bounds() {
        let config = TerrainConfig::default();

        assert_eq!(config.world_min(), Vec3::new(-16.0, 0.0, -16.0));
        assert_eq!(config.world_max(), Vec3::new(16.0, 0.0, 16.0));
    }

    #[test]
    fn contains_only_complete_surface_cells() {
        let config = TerrainConfig::default();
        let grids = GridConfig::default().into_grid_set().unwrap();

        assert!(config.contains_surface_cell(GridCoord::new(-32, 0, -32), grids));
        assert!(config.contains_surface_cell(GridCoord::new(31, 0, 31), grids));
        assert!(!config.contains_surface_cell(GridCoord::new(-33, 0, 0), grids));
        assert!(!config.contains_surface_cell(GridCoord::new(32, 0, 0), grids));
        assert!(!config.contains_surface_cell(GridCoord::new(0, 1, 0), grids));
    }

    #[test]
    fn checks_world_positions_against_metric_bounds() {
        let config = TerrainConfig::default();

        assert!(config.contains_world_xz(Vec3::new(-16.0, 5.0, -16.0)));
        assert!(config.contains_world_xz(Vec3::new(15.99, -5.0, 15.99)));
        assert!(!config.contains_world_xz(Vec3::new(-16.01, 0.0, 0.0)));
        assert!(!config.contains_world_xz(Vec3::new(16.0, 0.0, 0.0)));
    }

    #[test]
    fn rejects_footprints_that_cross_or_leave_bounds() {
        let config = TerrainConfig::default();
        let grids = GridConfig::default().into_grid_set().unwrap();

        assert!(config.contains_footprint(GridCoord::new(30, 0, 30), 2, 2, grids));
        assert!(!config.contains_footprint(GridCoord::new(31, 0, 31), 2, 2, grids));
        assert!(!config.contains_footprint(GridCoord::new(-33, 0, 0), 1, 1, grids));
        assert!(!config.contains_footprint(GridCoord::new(0, 0, 0), 0, 1, grids));
    }

    #[test]
    fn accepts_odd_meter_size_without_building_coupling() {
        let config = TerrainConfig {
            side_length_meters: 1,
        };
        let grids = GridConfig::default().into_grid_set().unwrap();

        assert_eq!(config.world_min(), Vec3::new(-0.5, 0.0, -0.5));
        assert_eq!(config.world_max(), Vec3::new(0.5, 0.0, 0.5));
        assert!(config.contains_surface_cell(GridCoord::new(-1, 0, -1), grids));
        assert!(config.contains_surface_cell(GridCoord::new(0, 0, 0), grids));
        assert!(!config.contains_surface_cell(GridCoord::new(1, 0, 0), grids));
    }
}
