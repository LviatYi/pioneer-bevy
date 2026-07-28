use crate::foundation::config::Validate;
use crate::grid::config::{GridConfig, cm_to_meters};
use crate::grid::{GridConfigValidationError, GridCoord};
use bevy::math::Vec3;
use bevy::prelude::Resource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grid {
    cell_size_cm: u32,
}

impl Grid {
    pub fn new(cell_size_cm: u32) -> Result<Self, GridConfigValidationError> {
        GridConfig {
            base_cell_size_cm: cell_size_cm,
            building_cell_size_cm: cell_size_cm,
        }
        .validated()?;

        Ok(Self { cell_size_cm })
    }

    pub const fn cell_size_cm(self) -> u32 {
        self.cell_size_cm
    }

    pub fn cell_size_meters(self) -> f32 {
        cm_to_meters(self.cell_size_cm)
    }

    pub fn cell_origin_world(self, coord: GridCoord) -> Vec3 {
        Vec3::new(
            grid_axis_to_world_origin(coord.x, self.cell_size_cm),
            grid_axis_to_world_origin(coord.y, self.cell_size_cm),
            grid_axis_to_world_origin(coord.z, self.cell_size_cm),
        )
    }

    pub fn cell_center_world(self, coord: GridCoord) -> Vec3 {
        let half_cell = self.cell_size_meters() * 0.5;
        self.cell_origin_world(coord) + Vec3::splat(half_cell)
    }

    pub fn world_to_cell_floor(self, position: Vec3) -> GridCoord {
        GridCoord::new(
            world_axis_to_grid_floor(position.x, self.cell_size_cm),
            world_axis_to_grid_floor(position.y, self.cell_size_cm),
            world_axis_to_grid_floor(position.z, self.cell_size_cm),
        )
    }

    pub fn world_to_nearest_cell(self, position: Vec3) -> GridCoord {
        GridCoord::new(
            world_axis_to_grid_round(position.x, self.cell_size_cm),
            world_axis_to_grid_round(position.y, self.cell_size_cm),
            world_axis_to_grid_round(position.z, self.cell_size_cm),
        )
    }

    pub fn snap_world_to_cell_origin(self, position: Vec3) -> Vec3 {
        self.cell_origin_world(self.world_to_nearest_cell(position))
    }

    pub fn snap_world_to_cell_center(self, position: Vec3) -> Vec3 {
        self.cell_center_world(self.world_to_nearest_cell(position))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Resource)]
pub struct GridSet {
    base: Grid,
    building: Grid,
}

impl GridSet {
    pub fn from_config(config: GridConfig) -> Result<Self, GridConfigValidationError> {
        let config = config.validated()?;

        Ok(Self {
            base: Grid {
                cell_size_cm: config.base_cell_size_cm,
            },
            building: Grid {
                cell_size_cm: config.building_cell_size_cm,
            },
        })
    }

    pub const fn base(self) -> Grid {
        self.base
    }

    pub const fn building(self) -> Grid {
        self.building
    }

    pub fn building_cell_size_in_base_cells(self) -> u32 {
        self.building.cell_size_cm / self.base.cell_size_cm
    }
}

fn grid_axis_to_world_origin(coord: i64, cell_size_cm: u32) -> f32 {
    coord as f32 * cm_to_meters(cell_size_cm)
}

fn world_axis_to_grid_floor(axis_meters: f32, cell_size_cm: u32) -> i64 {
    (axis_meters / cm_to_meters(cell_size_cm)).floor() as i64
}

fn world_axis_to_grid_round(axis_meters: f32, cell_size_cm: u32) -> i64 {
    (axis_meters / cm_to_meters(cell_size_cm)).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_base_and_building_grids_from_config() {
        let grid_set = GridConfig::default().into_grid_set().unwrap();

        assert_eq!(grid_set.base().cell_size_cm(), 5);
        assert_eq!(grid_set.building().cell_size_cm(), 50);
        assert_eq!(grid_set.building_cell_size_in_base_cells(), 10);
    }

    #[test]
    fn converts_grid_coord_to_bevy_world_units() {
        let grid = Grid::new(5).unwrap();

        assert_eq!(
            grid.cell_origin_world(GridCoord::new(2, -1, 10)),
            Vec3::new(0.1, -0.05, 0.5)
        );
        assert_eq!(
            grid.cell_center_world(GridCoord::new(0, 0, 0)),
            Vec3::splat(0.025)
        );
    }

    #[test]
    fn floors_world_position_to_containing_cell() {
        let grid = Grid::new(5).unwrap();

        assert_eq!(
            grid.world_to_cell_floor(Vec3::new(0.12, -0.01, 0.5)),
            GridCoord::new(2, -1, 10)
        );
    }

    #[test]
    fn snaps_world_position_to_nearest_building_cell_origin() {
        let grid = Grid::new(50).unwrap();

        assert_eq!(
            grid.snap_world_to_cell_origin(Vec3::new(1.24, 0.0, -1.26)),
            Vec3::new(1.0, 0.0, -1.5)
        );
    }
}
