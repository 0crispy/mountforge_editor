use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_mod_raycast::prelude::RaycastSource;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};


use bevy_atmosphere::prelude::*;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanOrbitCameraPlugin)
            .add_systems(Startup, setup)
            .add_systems(Update, update_atmosphere);
    }
}
fn setup(
    mut commands: Commands,
) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 10000.0, 8.0)
            .with_rotation(Quat::from_axis_angle(Vec3::X, -30.0 * (PI / 180.0))),
        ..default()
    });
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-20.0, 20., -20.0)
                .looking_at(Vec3::new(0., 10., 0.), Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                far: 100000.0,
                ..default()
            }),
            ..default()
        },
        PanOrbitCamera {
            button_orbit: MouseButton::Right,
            button_pan: MouseButton::Middle,
            ..Default::default()
        },
        RaycastSource::<()>::new_cursor(),
        AtmosphereCamera::default(),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.8, 0.8, 1.0),
        brightness: 0.5,
    });
}

fn update_atmosphere(
    mut atmosphere: AtmosphereMut<Nishita>,
    query: Query<&Transform, With<DirectionalLight>>,
) {
    atmosphere.sun_position = -query.single().forward();
    atmosphere.sun_intensity = 20.0;
}
