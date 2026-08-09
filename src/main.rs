mod bootstrap;
pub mod bsn_assets;
pub mod foundation;
pub mod grid;
mod movement;
pub mod terrain;

use crate::bootstrap::config_bootstrap;
use crate::bsn_assets::main_scene::main_scene;
use crate::movement::god::{god_camera, god_movement};
use crate::terrain::TerrainPlugin;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use foundation::asset::asset_guard::GitLfsAssetGuardPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(GitLfsAssetGuardPlugin::default())
        .add_plugins(DefaultPlugins);

    config_bootstrap(&mut app);

    app.add_plugins(TerrainPlugin)
        .add_systems(
            Startup,
            (main_scene.spawn(), god_camera.spawn(), lock_cursor),
        )
        .add_systems(Update, god_movement)
        .run();
}

fn lock_cursor(mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor_options) = cursor_options.single_mut() else {
        return;
    };

    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}
