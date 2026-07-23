mod asset_guard;
pub mod bsn_assets;
mod movement;

use crate::asset_guard::GitLfsAssetGuardPlugin;
use crate::bsn_assets::main_scene::main_scene;
use crate::movement::god::{god_camera, god_movement};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(GitLfsAssetGuardPlugin::default())
        .add_plugins(DefaultPlugins)
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
