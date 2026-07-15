use crate::bsn_assets::directional_light::directional_light;
use crate::bsn_assets::standard_cube::standard_cube_1m;
use bevy::prelude::*;

pub fn main_scene() -> impl SceneList {
    bsn_list![standard_cube_1m(), directional_light(),]
}
