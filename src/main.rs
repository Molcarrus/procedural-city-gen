mod building;
mod city_generator;
mod road;
mod terrain;
mod voronoi_city;

use bevy::{prelude::*, window::WindowResolution};

use crate::city_generator::CityGeneratorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Procedural City Generator".to_string(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CityGeneratorPlugin)
        .add_systems(Startup, setup_camera_and_light)
        .add_systems(Update, camera_controller)
        .run();
}

fn setup_camera_and_light(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(150.0, 200.0, 150.0).looking_at(Vec3::new(50.0, 0.0, 50.0), Vec3::Y),
        CameraController {
            speed: 50.0,
            sensitivity: 0.003,
            yaw: -std::f32::consts::FRAC_PI_4,
            pitch: -0.6,
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.95, 0.85),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 200.0,
        ..default()
    });
}

#[derive(Component)]
struct CameraController {
    speed: f32,
    sensitivity: f32,
    yaw: f32,
    pitch: f32,
}

fn camera_controller(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<(&mut Transform, &mut CameraController)>,
) {
    let (mut transform, mut controller) = query.single_mut().unwrap();

    if mouse_button.pressed(MouseButton::Right) {
        for event in mouse_motion.read() {
            controller.yaw -= event.delta.x * controller.sensitivity;
            controller.pitch -= event.delta.y * controller.sensitivity;
            controller.pitch = controller.pitch.clamp(-1.5, 1.5);
        }
    } else {
        mouse_motion.clear();
    }

    let yaw_rot = Quat::from_rotation_y(controller.yaw);
    let pitch_rot = Quat::from_rotation_x(controller.pitch);
    transform.rotation = yaw_rot * pitch_rot;

    let mut velocity = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    if keyboard.pressed(KeyCode::KeyW) {
        velocity += *forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        velocity -= *forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        velocity -= *right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        velocity += *right;
    }
    if keyboard.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        velocity -= Vec3::Y;
    }

    let speed_mult = if keyboard.pressed(KeyCode::ControlLeft) {
        3.0
    } else {
        1.0
    };

    if velocity.length_squared() > 0.0 {
        velocity = velocity.normalize();
    }

    transform.translation += velocity * controller.speed * speed_mult * time.delta_secs();
}
