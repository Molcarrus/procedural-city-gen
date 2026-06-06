use bevy::{
    DefaultPlugins,
    app::{App, AppExit, PluginGroup, Startup, Update},
    camera::ClearColor,
    color::Color,
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        message::MessageWriter,
        query::With,
        system::{Commands, Query, Res},
    },
    input::{ButtonInput, keyboard::KeyCode},
    light::DirectionalLight,
    math::{Vec2, Vec3, bounding::Aabb2d},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    transform::components::Transform,
    utils::default,
    window::{PrimaryWindow, Window, WindowPlugin},
};
use bevy_egui::EguiPlugin;
use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin};

mod config;
mod core;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: bevy::window::WindowMode::Windowed,
                resolution: bevy::window::WindowResolution::new(1920, 1080),
                ..Default::default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(WireframePlugin::default())
        .add_plugins(RtsCameraPlugin)
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Color::BLACK,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, (setup_camera, setup_light, maximize_window))
        .add_systems(Update, handle_exit)
        .run()
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        RtsCamera {
            bounds: Aabb2d::new(Vec2::ZERO, Vec2::new(200.0, 200.0)),
            min_angle: 0.66,
            height_max: 220.0,
            ..default()
        },
        RtsCameraControls {
            key_up: KeyCode::KeyW,
            key_down: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_rotate_left: KeyCode::F24,
            key_rotate_right: KeyCode::F23,
            pan_speed: 40.0,
            zoom_sensitivity: 0.15,
            edge_pan_width: 0.0,
            ..default()
        },
    ));
}

fn setup_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 1_700.0,
            ..default()
        },
        Transform::from_xyz(50_000.0, 50_000.0, 50_000.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn maximize_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in windows.iter_mut() {
        window.set_maximized(true);
    }
}

fn handle_exit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
