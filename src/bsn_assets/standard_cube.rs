use bevy::prelude::*;

pub fn standard_cube_1m() -> impl Scene {
    bsn! {
        #StandardCube1M
        WorldAssetRoot("models/standard/standard-cube-1m.glb#Scene0")
        Transform
    }
}
