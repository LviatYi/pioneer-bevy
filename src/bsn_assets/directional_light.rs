use bevy::prelude::*;

pub fn directional_light() -> impl Scene {
    bsn! {
        #Sun
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: true,
        }
        Transform {
            rotation: Quat::from_euler(EulerRot::XYZ, -1.0, -0.7, 0.0),
        }
    }
}
