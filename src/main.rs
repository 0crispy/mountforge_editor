mod camera;
mod details;
mod draw;
mod edit_chunks;
mod sculpt;
mod serialize;
mod terrain;
mod ui;

use std::ops::{Add, Mul};

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    window::PresentMode,
};


use bevy_atmosphere::prelude::*;

use bevy_mod_raycast::prelude::{DeferredRaycastingPlugin, RaycastPluginState};
use details::DetailsPlugin;
use draw::DrawPlugin;
use edit_chunks::EditChunksPlugin;
use sculpt::SculptPlugin;
use serialize::SerializePlugin;
use terrain::TerrainPlugin;
use ui::TerrainUiPlugin;

const VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Reflect, Clone)]
struct MyRaycastSet;
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Mountforge".to_string(),
                        present_mode: PresentMode::AutoNoVsync,
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(AssetPlugin {
                    file_path: "data".to_string(),
                    ..Default::default()
                }),
        )
        .insert_resource(Msaa::Sample4)
        .add_plugins((
            DeferredRaycastingPlugin::<()>::default(),
            AtmospherePlugin,
            camera::CameraPlugin,
            TerrainUiPlugin,
            TerrainPlugin,
            EditChunksPlugin,
            SculptPlugin,
            DrawPlugin,
            DetailsPlugin,
            SerializePlugin,
            WireframePlugin,
        ))
        .insert_resource(AtmosphereModel::default())
        .insert_resource(RaycastPluginState::<()>::default())
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Color::BLACK,
        })
        // .add_systems(Update,(
        //     //update_raycast_with_cursor,
        //     //whatever,
        // ))
        .insert_resource(DebugInformation::default())
        .run();
}

#[derive(Resource)]
pub struct DebugInformation {}
impl Default for DebugInformation {
    fn default() -> Self {
        Self {}
    }
}

fn sample_vec<T: Mul<f32, Output = T> + Add<Output = T> + Copy>(
    x: f32,
    y: f32,
    values: &Vec<T>,
    size: usize,
) -> T {
    let x_source = x * size as f32;
    let y_source = y * size as f32;

    let x_floor = (x_source.floor() as usize).clamp(0, size - 1);
    let y_floor = (y_source.floor() as usize).clamp(0, size - 1);
    let x_ceil = (x_source.ceil() as usize).clamp(0, size - 1);
    let y_ceil = (y_source.ceil() as usize).clamp(0, size - 1);

    let x_fraction = x_source - x_floor as f32;
    let y_fraction = y_source - y_floor as f32;

    let top_left = values[x_floor + y_floor * size];
    let top_right = values[x_ceil + y_floor * size];
    let bottom_left = values[x_floor + y_ceil * size];
    let bottom_right = values[x_ceil + y_ceil * size];

    let top_interpolation = top_left + (top_right + top_left * (-1.0)) * x_fraction;
    let bottom_interpolation = bottom_left + (bottom_right + bottom_left * (-1.0)) * x_fraction;

    top_interpolation + (bottom_interpolation + top_interpolation * (-1.0)) * y_fraction
}
pub fn resize_vector<T: Mul<f32, Output = T> + Add<Output = T> + Default + Copy>(
    source_vector: &Vec<T>,
    size: usize,
    target_size: usize,
) -> Vec<T> {
    let mut target_vector: Vec<T> = vec![T::default(); target_size * target_size];

    for i in 0..target_size {
        for j in 0..target_size {
            let x = j as f32 / target_size as f32;
            let y = i as f32 / target_size as f32;

            target_vector[i + j * target_size] = sample_vec(x, y, source_vector, size);
        }
    }
    target_vector
}
