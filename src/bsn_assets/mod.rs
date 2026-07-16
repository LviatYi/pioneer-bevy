//TODO_LviatYi: The structure of BSN assets may need to be reorganized.
pub mod directional_light;
pub mod main_scene;
mod optional_world_asset;
pub mod standard_cube;

use bevy::prelude::*;
use optional_world_asset::{load_optional_world_assets, spawn_loaded_optional_world_assets};

pub struct BsnAssetsPlugin;

impl Plugin for BsnAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                load_optional_world_assets,
                spawn_loaded_optional_world_assets,
            )
                .chain(),
        );
    }
}
