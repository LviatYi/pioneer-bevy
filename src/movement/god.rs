use bevy::input::ButtonInput;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::AmbientLight;
use bevy::math::Vec3;
use bevy::prelude::*;

const STARTING_YAW: f32 = 0.0;
const STARTING_PITCH: f32 = -0.3;
const MOVE_SPEED: f32 = 5.0;
const FAST_MULTIPLIER: f32 = 4.0;
const FLASH_DISTANCE: f32 = 8.0;

const STARTING_POSITION: Vec3 = Vec3::new(0.0, 2.0, 6.0);
const LOOK_SENSITIVITY: f32 = 0.002;
const RESET_HOLD_SECONDS: f32 = 0.8;

#[derive(Component, Clone)]
pub struct GodCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub reset_timer: Timer,
}

impl Default for GodCamera {
    fn default() -> Self {
        Self {
            yaw: STARTING_YAW,
            pitch: STARTING_PITCH,
            reset_timer: Timer::from_seconds(RESET_HOLD_SECONDS, TimerMode::Once),
        }
    }
}

pub fn god_camera() -> impl Scene {
    bsn! {
        #GodCamera
        Camera3d
        AmbientLight {
            color: Color::WHITE,
            brightness: 350.0,
            affects_lightmapped_meshes: true,
        }
        Transform {
            translation: STARTING_POSITION,
            rotation: camera_rotation(STARTING_YAW, STARTING_PITCH),
        }
        GodCamera
    }
}

pub fn god_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut cameras: Query<(&mut Transform, &mut GodCamera)>,
) {
    let Ok((mut transform, mut camera)) = cameras.single_mut() else {
        return;
    };

    let mouse_delta = mouse_motion.delta;
    camera.yaw -= mouse_delta.x * LOOK_SENSITIVITY;
    camera.pitch = (camera.pitch - mouse_delta.y * LOOK_SENSITIVITY).clamp(-1.54, 1.54);
    transform.rotation = camera_rotation(camera.yaw, camera.pitch);

    let mut movement = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    if keyboard.pressed(KeyCode::KeyW) {
        movement += *forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement -= *forward;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += *right;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement -= *right;
    }
    if keyboard.pressed(KeyCode::Space) {
        movement += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::AltLeft) {
        movement -= Vec3::Y;
    }

    let speed_multiplier = if keyboard.pressed(KeyCode::ShiftLeft) {
        FAST_MULTIPLIER
    } else {
        1.0
    };

    if movement != Vec3::ZERO {
        transform.translation +=
            movement.normalize() * MOVE_SPEED * speed_multiplier * time.delta_secs();
    }

    if keyboard.just_pressed(KeyCode::KeyF) {
        transform.translation += *forward * FLASH_DISTANCE * speed_multiplier;
    }

    if keyboard.pressed(KeyCode::KeyR) {
        camera.reset_timer.tick(time.delta());
        if camera.reset_timer.just_finished() {
            camera.yaw = STARTING_YAW;
            camera.pitch = STARTING_PITCH;
            camera.reset_timer.reset();
            transform.translation = STARTING_POSITION;
            transform.rotation = camera_rotation(camera.yaw, camera.pitch);
        }
    } else {
        camera.reset_timer.reset();
    }
}

fn camera_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch)
}
