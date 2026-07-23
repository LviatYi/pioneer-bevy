use bevy::prelude::*;

const STANDARD_CUBE_1M_PATH: &str = "models/standard/standard-cube-1m.glb#Scene0";

pub fn standard_cube_1m() -> impl Scene {
    bsn! {
        #StandardCube1M
        WorldAssetRoot(STANDARD_CUBE_1M_PATH)
        Transform
    }
}
