use bevy::prelude::*;
use bevy_mod_raycast::deferred::RaycastSource;

use crate::{
    resize_vector,
    terrain::MasterTerrain,
    ui::{EditInfo, EditMode, SculptType, UiHovered},
};
pub struct SculptPlugin;
impl Plugin for SculptPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sculpt);
    }
}

fn sculpt(
    mut edit_info: ResMut<EditInfo>,
    raycast_source: Query<&RaycastSource<()>>,
    master_terrain: Res<MasterTerrain>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    ui_hovered: Res<UiHovered>,
) {
    if !master_terrain.loaded {
        return;
    }
    if let EditMode::Sculpt = &edit_info.edit_mode {
        let raycast_source = raycast_source.single();
        for (_, intersection) in raycast_source.intersections() {
            if mouse.pressed(MouseButton::Left) && !ui_hovered.0 {
                let pos = intersection.position();
                if edit_info.sculpt_info.auto_height && mouse.just_pressed(MouseButton::Left) {
                    edit_info.sculpt_info.set_height = pos.y;
                }
                let pos = master_terrain.vec2_to_world_pos(Vec2::new(pos.x, pos.z));
                if edit_info.sculpt_info.brush_info.selected_brush.is_none() {
                    return;
                }
                let size = edit_info.sculpt_info.brush_info.size;
                if edit_info
                    .sculpt_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map_size
                    != size
                {
                    let brush = edit_info
                        .sculpt_info
                        .brush_info
                        .selected_brush
                        .as_mut()
                        .unwrap();
                    brush.sample_map =
                        resize_vector(&brush.map, brush.map_size as usize, size as usize);
                    brush.sample_map_size = size;
                }
                let sample_map = &edit_info
                    .sculpt_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map;
                let strength = edit_info.sculpt_info.brush_info.strength;
                let set_height = edit_info.sculpt_info.set_height;
                let avg_height = match edit_info.sculpt_info.sculpt_type {
                    SculptType::Smooth => {
                        let mut avg = 0.0;
                        let mut avg_div = f32::EPSILON;
                        for x in 0..size {
                            for y in 0..size {
                                let x_f32 = x as f32 - size as f32 * 0.5;
                                let y_f32 = y as f32 - size as f32 * 0.5;
                                let brush_sample = sample_map[(x + y * size) as usize];
                                let world_pos =
                                    pos + master_terrain.vec2_to_world_pos(Vec2::new(x_f32, y_f32));
                                avg += master_terrain.get_height(world_pos) * brush_sample;
                                avg_div += brush_sample;
                            }
                        }
                        Some(avg / avg_div)
                    }
                    _ => None,
                };
                for x in 0..size {
                    for y in 0..size {
                        let x_f32 = x as f32 - size as f32 * 0.5;
                        let y_f32 = y as f32 - size as f32 * 0.5;
                        let brush_sample = sample_map[(x + y * size) as usize];
                        let world_pos =
                            pos + master_terrain.vec2_to_world_pos(Vec2::new(x_f32, y_f32));
                        let delta = if keys.pressed(KeyCode::ControlLeft) {
                            -time.delta_seconds()
                        } else {
                            time.delta_seconds()
                        };
                        match &edit_info.sculpt_info.sculpt_type {
                            SculptType::RaiseLower => {
                                master_terrain
                                    .add_height(world_pos, delta * 500.0 * brush_sample * strength);
                            }
                            SculptType::SetHeight => {
                                let current_height = master_terrain.get_height(world_pos);
                                let diff = set_height - current_height;
                                let clamped_diff = diff
                                    .abs()
                                    .min(500.0 * brush_sample * strength * time.delta_seconds())
                                    * diff.signum();
                                master_terrain.set_height(world_pos, current_height + clamped_diff);
                            }
                            SculptType::Smooth => {
                                let current_height = master_terrain.get_height(world_pos);
                                let diff = avg_height.unwrap() - current_height;
                                let clamped_diff = diff
                                    .abs()
                                    .min(500.0 * brush_sample * strength * time.delta_seconds())
                                    * diff.signum();
                                master_terrain.set_height(world_pos, current_height + clamped_diff);
                            }
                        }
                    }
                }
            }
        }
    }
}
