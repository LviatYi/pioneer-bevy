use crate::bsn_assets::directional_light::directional_light;
use crate::bsn_assets::origin_cube::origin_cube;
use bevy::prelude::*;

pub fn main_scene() -> impl SceneList {
    bsn_list![origin_cube(), directional_light(),]
}
