use crate::foundation::config::{ConfigLoadError, ConfigPath, Validate, load_ron_config};
use crate::grid::GridSet;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub const DEFAULT_BASE_CELL_SIZE_CM: u32 = 5;
pub const DEFAULT_BUILDING_CELL_SIZE_CM: u32 = 50;
pub const DEFAULT_GRID_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct GridConfig {
    pub base_cell_size_cm: u32,
    pub building_cell_size_cm: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            base_cell_size_cm: DEFAULT_BASE_CELL_SIZE_CM,
            building_cell_size_cm: DEFAULT_BUILDING_CELL_SIZE_CM,
        }
    }
}

impl GridConfig {
    pub fn load_default_ron_file() -> Result<Self, ConfigLoadError<GridConfigValidationError>> {
        Self::load_from_ron_file(DEFAULT_GRID_CONFIG_PATH.file_system_path())
    }

    pub fn load_from_ron_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, ConfigLoadError<GridConfigValidationError>> {
        load_ron_config(path)
    }

    pub fn validate(self) -> Result<Self, GridConfigValidationError> {
        <Self as Validate>::validated(self)
    }

    pub fn base_cell_size_meters(self) -> f32 {
        cm_to_meters(self.base_cell_size_cm)
    }

    pub fn building_cell_size_meters(self) -> f32 {
        cm_to_meters(self.building_cell_size_cm)
    }

    pub fn base_cells_per_building_cell(self) -> u32 {
        self.building_cell_size_cm / self.base_cell_size_cm
    }

    pub fn into_grid_set(self) -> Result<GridSet, GridConfigValidationError> {
        GridSet::from_config(self)
    }
}

impl Validate for GridConfig {
    type Error = GridConfigValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        validate_positive("base_cell_size_cm", self.base_cell_size_cm)?;
        validate_positive("building_cell_size_cm", self.building_cell_size_cm)?;

        if self.building_cell_size_cm % self.base_cell_size_cm != 0 {
            return Err(GridConfigValidationError::BuildingCellNotMultipleOfBase {
                base_cell_size_cm: self.base_cell_size_cm,
                building_cell_size_cm: self.building_cell_size_cm,
            });
        }

        Ok(())
    }
}

fn validate_positive(field: &'static str, value: u32) -> Result<(), GridConfigValidationError> {
    if value == 0 {
        return Err(GridConfigValidationError::CellSizeMustBePositive { field });
    }

    Ok(())
}

pub(crate) fn cm_to_meters(value_cm: u32) -> f32 {
    value_cm as f32 / 100.0
}

#[derive(Debug, Error)]
pub enum GridConfigValidationError {
    #[error("grid config field `{field}` must be greater than 0")]
    CellSizeMustBePositive { field: &'static str },
    #[error(
        "building cell size ({building_cell_size_cm}cm) must be a multiple of base cell size ({base_cell_size_cm}cm)"
    )]
    BuildingCellNotMultipleOfBase {
        base_cell_size_cm: u32,
        building_cell_size_cm: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_current_design_values() {
        let config = GridConfig::default();

        assert_eq!(config.base_cell_size_cm, 5);
        assert_eq!(config.building_cell_size_cm, 50);
        assert_eq!(config.base_cell_size_meters(), 0.05);
        assert_eq!(config.building_cell_size_meters(), 0.5);
        assert_eq!(config.base_cells_per_building_cell(), 10);
    }

    #[test]
    fn loads_default_grid_config_file() {
        assert_eq!(
            GridConfig::load_default_ron_file().unwrap(),
            GridConfig::default()
        );
    }

    #[test]
    fn rejects_zero_cell_size() {
        let error = GridConfig {
            base_cell_size_cm: 0,
            building_cell_size_cm: 50,
        }
        .validate()
        .unwrap_err();

        assert!(matches!(
            error,
            GridConfigValidationError::CellSizeMustBePositive {
                field: "base_cell_size_cm"
            }
        ));
    }

    #[test]
    fn rejects_building_cell_that_is_not_aligned_to_base_grid() {
        let error = GridConfig {
            base_cell_size_cm: 6,
            building_cell_size_cm: 50,
        }
        .validate()
        .unwrap_err();

        assert!(matches!(
            error,
            GridConfigValidationError::BuildingCellNotMultipleOfBase {
                base_cell_size_cm: 6,
                building_cell_size_cm: 50
            }
        ));
    }
}
