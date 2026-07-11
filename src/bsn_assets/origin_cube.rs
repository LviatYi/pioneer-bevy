use bevy::prelude::*;

pub fn origin_cube() -> impl Scene {
    bsn! {
        #OriginCube
        Mesh3d(asset_value(Cuboid::from_size(Vec3::ONE)))
        MeshMaterial3d::<StandardMaterial>(asset_value(StandardMaterial {
            base_color: Color::srgb(0.8, 0.85, 0.9),
            ..default()
        }))
        Transform
    }
}
