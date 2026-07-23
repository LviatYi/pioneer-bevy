use bevy::asset::io::{
    AssetReaderError, AssetSource, AssetSourceBuilder, AssetSourceId, ErasedAssetReader,
    PathStream, Reader, VecReader,
};
use bevy::asset::{AssetApp, AsyncReadExt, AsyncSeekExt};
use bevy::prelude::*;
use bevy::tasks::BoxedFuture;
use std::io::{Error, ErrorKind, SeekFrom};
use std::path::Path;
use std::sync::Arc;

const GIT_LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";

#[derive(Clone)]
pub struct GitLfsAssetGuardPlugin {
    asset_root: String,
}

impl Default for GitLfsAssetGuardPlugin {
    fn default() -> Self {
        Self {
            asset_root: "assets".to_owned(),
        }
    }
}

impl Plugin for GitLfsAssetGuardPlugin {
    fn build(&self, app: &mut App) {
        let asset_root = self.asset_root.clone();
        let mut default_reader = AssetSource::get_default_reader(asset_root.clone());
        let guarded_source = AssetSourceBuilder::platform_default(&asset_root, None)
            .with_reader(move || Box::new(GitLfsGuardedAssetReader::new(default_reader())));

        app.register_asset_source(AssetSourceId::Default, guarded_source);
    }
}

struct GitLfsGuardedAssetReader {
    inner: Box<dyn ErasedAssetReader>,
}

impl GitLfsGuardedAssetReader {
    fn new(inner: Box<dyn ErasedAssetReader>) -> Self {
        Self { inner }
    }
}

impl ErasedAssetReader for GitLfsGuardedAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        Box::pin(async move {
            let reader = self.inner.read(path).await?;
            reject_git_lfs_pointer(path, reader).await
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        self.inner.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        self.inner.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        self.inner.is_directory(path)
    }

    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetReaderError>> {
        self.inner.read_meta_bytes(path)
    }
}

async fn reject_git_lfs_pointer<'a>(
    path: &Path,
    mut reader: Box<dyn Reader + 'a>,
) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
    if reader.seekable().is_ok() {
        let prefix = read_prefix(&mut *reader).await?;
        if is_git_lfs_pointer(&prefix) {
            return Err(git_lfs_pointer_error(path));
        }

        reader
            .seekable()
            .map_err(|_| {
                AssetReaderError::Io(Arc::new(Error::new(
                    ErrorKind::Other,
                    "asset reader stopped supporting seek after prefix validation",
                )))
            })?
            .seek(SeekFrom::Start(0))
            .await?;
        return Ok(reader);
    }

    let mut bytes = Vec::new();
    Reader::read_to_end(&mut reader, &mut bytes).await?;
    if is_git_lfs_pointer(&bytes) {
        return Err(git_lfs_pointer_error(path));
    }

    Ok(Box::new(VecReader::new(bytes)))
}

async fn read_prefix(reader: &mut dyn Reader) -> Result<Vec<u8>, AssetReaderError> {
    let mut prefix = vec![0; GIT_LFS_POINTER_PREFIX.len()];
    let mut read_len = 0;

    while read_len < prefix.len() {
        let bytes_read = reader.read(&mut prefix[read_len..]).await?;
        if bytes_read == 0 {
            break;
        }
        read_len += bytes_read;
    }

    prefix.truncate(read_len);
    Ok(prefix)
}

fn is_git_lfs_pointer(bytes: &[u8]) -> bool {
    bytes.starts_with(GIT_LFS_POINTER_PREFIX)
}

fn git_lfs_pointer_error(path: &Path) -> AssetReaderError {
    AssetReaderError::Io(Arc::new(Error::new(
        ErrorKind::InvalidData,
        format!(
            "asset `{}` is a Git LFS pointer, not downloaded content",
            path.display()
        ),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_git_lfs_pointer() {
        assert!(is_git_lfs_pointer(
            b"version https://git-lfs.github.com/spec/v1\noid sha256:test\nsize 123\n"
        ));
    }

    #[test]
    fn accepts_non_lfs_content() {
        assert!(!is_git_lfs_pointer(b"glTF\x02\0\0\0"));
    }
}
