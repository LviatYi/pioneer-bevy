use crate::bsn_assets::directional_light::directional_light;
use bevy::prelude::*;

pub fn main_scene() -> impl SceneList {
    bsn_list![directional_light(),]
}
