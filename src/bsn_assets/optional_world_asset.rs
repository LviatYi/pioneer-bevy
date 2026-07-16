use bevy::asset::{LoadState, RecursiveDependencyLoadState};
use bevy::prelude::*;
use std::path::Path;

const GIT_LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";

#[derive(Component, Clone, Default)]
pub struct OptionalWorldAssetRoot {
    pub path: &'static str,
}

impl OptionalWorldAssetRoot {
    pub const fn new(path: &'static str) -> Self {
        Self { path }
    }
}

#[derive(Component)]
pub(super) struct LoadingOptionalWorldAsset {
    handle: Handle<WorldAsset>,
}

pub(super) fn load_optional_world_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Query<(Entity, &OptionalWorldAssetRoot), Added<OptionalWorldAssetRoot>>,
) {
    for (entity, asset) in &assets {
        if let Err(reason) = validate_local_asset(asset.path) {
            warn!("Skipping optional world asset {}: {reason}", asset.path);
            commands.entity(entity).despawn();
            continue;
        }

        commands.entity(entity).insert(LoadingOptionalWorldAsset {
            handle: asset_server.load(asset.path),
        });
    }
}

pub(super) fn spawn_loaded_optional_world_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Query<(Entity, &OptionalWorldAssetRoot, &LoadingOptionalWorldAsset)>,
) {
    for (entity, asset, loading) in &assets {
        if asset_server.is_loaded_with_dependencies(&loading.handle) {
            commands
                .entity(entity)
                .insert(WorldAssetRoot(loading.handle.clone()))
                .remove::<LoadingOptionalWorldAsset>();
            continue;
        }

        if let LoadState::Failed(error) = asset_server.load_state(&loading.handle) {
            warn!("Skipping optional world asset {}: {error}", asset.path);
            commands.entity(entity).despawn();
            continue;
        }

        if let RecursiveDependencyLoadState::Failed(error) =
            asset_server.recursive_dependency_load_state(&loading.handle)
        {
            warn!("Skipping optional world asset {}: {error}", asset.path);
            commands.entity(entity).despawn();
        }
    }
}

fn validate_local_asset(asset_path: &str) -> Result<(), String> {
    let asset_file = asset_path
        .split_once('#')
        .map_or(asset_path, |(path, _)| path);
    let local_path = Path::new("assets").join(asset_file);

    validate_local_asset_file(&local_path)
}

fn validate_local_asset_file(local_path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(local_path).map_err(|error| {
        format!(
            "asset file is unavailable at {}: {error}",
            local_path.display()
        )
    })?;

    if bytes.starts_with(GIT_LFS_POINTER_PREFIX) {
        return Err(format!(
            "asset file at {} is a Git LFS pointer, not downloaded content",
            local_path.display()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_git_lfs_pointer_files() {
        let test_dir = Path::new("target").join("test-assets");
        std::fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("lfs-pointer.glb");
        std::fs::write(
            &test_file,
            b"version https://git-lfs.github.com/spec/v1\noid sha256:test\nsize 123\n",
        )
        .unwrap();

        let error = validate_local_asset_file(&test_file).unwrap_err();

        std::fs::remove_file(&test_file).unwrap();
        assert!(error.contains("Git LFS pointer"));
    }
}
