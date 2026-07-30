use crate::foundation::config::{ConfigPath, ConfigRuntime, Validate};
use crate::grid::GridSet;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_BASE_CELL_SIZE_CM: u32 = 5;
pub const DEFAULT_BUILDING_CELL_SIZE_CM: u32 = 50;
pub const DEFAULT_GRID_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Resource, Serialize)]
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

impl ConfigRuntime for GridConfig {
    type Runtime = GridSet;
    type Error = GridConfigValidationError;

    fn build_runtime(&self) -> Result<Self::Runtime, Self::Error> {
        GridSet::from_config(*self)
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
    use crate::foundation::config::{
        ConfigAsset, ConfigHandle, ConfigSource, ConfigState, ConfigStatus, RonConfigFormatLoader,
        RonConfigPlugin, load_validated_config_bytes,
    };
    use bevy::asset::{AssetLoadError, AssetLoadFailedEvent, AssetPlugin};
    use bevy::ecs::message::Messages;
    use bevy::prelude::*;

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
        let bytes = std::fs::read(DEFAULT_GRID_CONFIG_PATH.file_system_path()).unwrap();

        assert_eq!(
            load_validated_config_bytes::<GridConfig, RonConfigFormatLoader<GridConfig>>(
                &bytes,
                DEFAULT_GRID_CONFIG_PATH.asset_path(),
            )
            .unwrap(),
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

    #[test]
    fn plugin_registers_grid_config_as_asset() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<GridConfig>::new(DEFAULT_GRID_CONFIG_PATH),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();

        assert_eq!(handle.path(), DEFAULT_GRID_CONFIG_PATH);
        assert!(app.world().get_resource::<ConfigState<GridSet>>().is_none());
        assert!(
            app.world()
                .get_resource::<ConfigState<GridConfig>>()
                .is_some()
        );
    }

    #[test]
    fn config_handle_reads_registered_asset() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<GridConfig>::new(DEFAULT_GRID_CONFIG_PATH),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let id = handle.id();
        let assets = app
            .world_mut()
            .resource_mut::<Assets<ConfigAsset<GridConfig>>>();
        assets
            .into_inner()
            .insert(id, ConfigAsset::new(GridConfig::default()))
            .unwrap();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let assets = app.world().resource::<Assets<ConfigAsset<GridConfig>>>();
        assert_eq!(handle.get(assets), Some(&GridConfig::default()));

        app.world_mut()
            .resource_scope(|world, handle: Mut<ConfigHandle<GridConfig>>| {
                let mut assets = world.resource_mut::<Assets<ConfigAsset<GridConfig>>>();
                let mut config = handle.get_mut(&mut assets).unwrap();
                config.value_mut().building_cell_size_cm = 100;
            });

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let assets = app.world().resource::<Assets<ConfigAsset<GridConfig>>>();
        assert_eq!(handle.get(assets).unwrap().building_cell_size_cm, 100);
    }

    #[test]
    fn loaded_grid_config_is_applied_as_grid_set_resource() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<GridConfig>::new(DEFAULT_GRID_CONFIG_PATH)
                .with_validation()
                .with_runtime()
                .fallback_to_default(),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let id = handle.id();
        let assets = app
            .world_mut()
            .resource_mut::<Assets<ConfigAsset<GridConfig>>>();
        assets
            .into_inner()
            .insert(id, ConfigAsset::new(GridConfig::default()))
            .unwrap();

        app.update();
        app.update();

        let grid_set = app.world().resource::<GridSet>();
        let state = app.world().resource::<ConfigState<GridSet>>();

        assert_eq!(grid_set.building_cell_size_in_base_cells(), 10);
        assert!(state.is_resolved());
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::Asset(DEFAULT_GRID_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn fallback_grid_config_uses_default_when_asset_load_fails() {
        const MISSING_GRID_CONFIG_PATH: ConfigPath = ConfigPath::new("config/missing-grid.ron");

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<GridConfig>::new(MISSING_GRID_CONFIG_PATH)
                .with_validation()
                .with_runtime()
                .fallback_to_default(),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let id = handle.id();
        app.world_mut()
            .resource_mut::<Messages<AssetLoadFailedEvent<ConfigAsset<GridConfig>>>>()
            .write(AssetLoadFailedEvent {
                id,
                path: MISSING_GRID_CONFIG_PATH.asset_path().into(),
                error: AssetLoadError::MissingAssetLoader {
                    asset_type_id: None,
                    asset_path: MISSING_GRID_CONFIG_PATH.asset_path().to_owned(),
                },
            });

        app.update();
        app.update();

        let config = app.world().resource::<GridConfig>();
        let grid_set = app.world().resource::<GridSet>();
        let state = app.world().resource::<ConfigState<GridSet>>();

        assert_eq!(config, &GridConfig::default());
        assert_eq!(grid_set.building_cell_size_in_base_cells(), 10);
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::DefaultFallback(MISSING_GRID_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn fallback_grid_config_without_runtime_resolves_grid_config_resource() {
        const MISSING_GRID_CONFIG_PATH: ConfigPath = ConfigPath::new("config/missing-grid.ron");

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            RonConfigPlugin::<GridConfig>::new(MISSING_GRID_CONFIG_PATH)
                .with_validation()
                .fallback_to_default(),
        ));
        app.update();

        let handle = app.world().resource::<ConfigHandle<GridConfig>>();
        let id = handle.id();
        app.world_mut()
            .resource_mut::<Messages<AssetLoadFailedEvent<ConfigAsset<GridConfig>>>>()
            .write(AssetLoadFailedEvent {
                id,
                path: MISSING_GRID_CONFIG_PATH.asset_path().into(),
                error: AssetLoadError::MissingAssetLoader {
                    asset_type_id: None,
                    asset_path: MISSING_GRID_CONFIG_PATH.asset_path().to_owned(),
                },
            });

        app.update();

        let config = app.world().resource::<GridConfig>();
        let state = app.world().resource::<ConfigState<GridConfig>>();

        assert_eq!(config, &GridConfig::default());
        assert!(app.world().get_resource::<ConfigState<GridSet>>().is_none());
        assert_eq!(
            state.status(),
            &ConfigStatus::Resolved {
                source: ConfigSource::DefaultFallback(MISSING_GRID_CONFIG_PATH)
            }
        );
    }
}
