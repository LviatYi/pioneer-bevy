use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigPath {
    asset_path: &'static str,
}

impl ConfigPath {
    pub const fn new(asset_path: &'static str) -> Self {
        Self { asset_path }
    }

    pub const fn asset_path(self) -> &'static str {
        self.asset_path
    }

    pub fn file_system_path(self) -> PathBuf {
        Path::new("assets").join(self.asset_path)
    }
}
