use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct GridCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl GridCoord {
    pub const ZERO: Self = Self::new(0, 0, 0);

    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }
}
