use crate::foundation::config::{ConfigPath, RonConfigPlugin};
use crate::grid::GridConfig;
use crate::terrain::TerrainConfig;
use crate::terrain::landform_algorithm::plains::PlainsLandformConfig;
use bevy::app::App;

pub const DEFAULT_GRID_CONFIG_PATH: ConfigPath = ConfigPath::new("config/grid.ron");
pub const DEFAULT_TERRAIN_CONFIG_PATH: ConfigPath = ConfigPath::new("config/terrain.ron");
pub const DEFAULT_PLAINS_LANDFORM_CONFIG_PATH: ConfigPath =
    ConfigPath::new("config/plains-landform.ron");

pub fn config_bootstrap(app: &mut App) -> &mut App {
    app.add_plugins(
        RonConfigPlugin::<GridConfig>::new(DEFAULT_GRID_CONFIG_PATH)
            .with_validation()
            .with_runtime_transformer()
            .fallback_to_default(),
    )
    .add_plugins(
        RonConfigPlugin::<TerrainConfig>::new(DEFAULT_TERRAIN_CONFIG_PATH)
            .with_validation()
            .fallback_to_default(),
    )
    .add_plugins(
        RonConfigPlugin::<PlainsLandformConfig>::new(DEFAULT_PLAINS_LANDFORM_CONFIG_PATH)
            .with_validation()
            .with_runtime_transformer()
            .fallback_to_default(),
    )
}
