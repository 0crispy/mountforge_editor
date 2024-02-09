use std::{
    collections::HashMap,
    ops::{Add, Mul, Sub},
};

use bevy::prelude::*;
use bevy_mod_raycast::deferred::RaycastSource;

use crate::{
    resize_vector,
    terrain::MasterTerrain,
    ui::{EditInfo, EditMode, UiHovered},
};

pub struct DrawPlugin;
impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw);
    }
}

fn draw(
    mut edit_info: ResMut<EditInfo>,
    raycast_source: Query<&RaycastSource<()>>,
    master_terrain: Res<MasterTerrain>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
    ui_hovered: Res<UiHovered>,
    mut images: ResMut<Assets<Image>>,
) {
    if !master_terrain.loaded {
        return;
    }
    if let EditMode::Draw = &edit_info.edit_mode {
        let raycast_source = raycast_source.single();
        for (_, intersection) in raycast_source.intersections() {
            if mouse.pressed(MouseButton::Left) && !ui_hovered.0 {
                let intersection_pos = intersection.position();
                let pixel_pos = master_terrain
                    .vec2_to_pixel_pos(Vec2::new(intersection_pos.x, intersection_pos.z));
                if edit_info.draw_info.brush_info.selected_brush.is_none() {
                    return;
                }
                if edit_info
                    .draw_info
                    .draw_texture_info
                    .selected_texture
                    .is_none()
                {
                    return;
                }
                let scale = edit_info.draw_info.draw_texture_info.scale;
                let size = edit_info.draw_info.brush_info.size;
                if edit_info
                    .draw_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map_size
                    != size
                {
                    let brush = edit_info
                        .draw_info
                        .brush_info
                        .selected_brush
                        .as_mut()
                        .unwrap();
                    brush.sample_map =
                        resize_vector(&brush.map, brush.map_size as usize, size as usize);
                    brush.sample_map_size = size;
                }
                if edit_info
                    .draw_info
                    .draw_texture_info
                    .selected_texture
                    .as_ref()
                    .unwrap()
                    .sample_map_size
                    != scale
                {
                    let draw_texture = edit_info
                        .draw_info
                        .draw_texture_info
                        .selected_texture
                        .as_mut()
                        .unwrap();
                    draw_texture.sample_map = resize_vector(
                        &draw_texture.map,
                        draw_texture.map_size as usize,
                        scale as usize,
                    );
                    draw_texture.sample_map_size = scale;
                }
                let strength_sample_map = &edit_info
                    .draw_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map;
                let texture_sample_map = &edit_info
                    .draw_info
                    .draw_texture_info
                    .selected_texture
                    .as_ref()
                    .unwrap()
                    .sample_map;
                let strength = edit_info.draw_info.brush_info.strength;
                let p_per_tile = master_terrain.pixels_per_tile();
                let p_size = size * p_per_tile as u32;

                let mut image_map: HashMap<IVec2, Option<Vec<(Color, usize)>>> = HashMap::new();

                for p_x in 0..p_size {
                    for p_y in 0..p_size {
                        let p_x_f32 = p_x as f32 - p_size as f32 * 0.5;
                        let p_y_f32 = p_y as f32 - p_size as f32 * 0.5;
                        let x_f32 = p_x_f32 / p_per_tile as f32;
                        let y_f32 = p_y_f32 / p_per_tile as f32;
                        let (x, y) = (p_x / p_per_tile as u32, p_y / p_per_tile as u32);
                        let pixel_pos =
                            pixel_pos + master_terrain.vec2_to_pixel_pos(Vec2::new(x_f32, y_f32));
                        let chunk_pos = master_terrain.pixel_to_chunk_pos(pixel_pos);
                        if let Some(img) = image_map.get(&chunk_pos) {
                            if img.is_none() {
                                continue;
                            }
                        } else {
                            let textures = &master_terrain.texture_map.textures;
                            let handle = if let Some(handle) = textures.get(&chunk_pos) {
                                handle
                            } else {
                                image_map.insert(chunk_pos, None);
                                continue;
                            };
                            if images.get_mut(handle).is_none() {
                                image_map.insert(chunk_pos, None);
                                continue;
                            }
                            image_map.insert(chunk_pos, Some(Vec::new()));
                        }

                        let texture_sample = sample_repeating(
                            pixel_pos.x as i32,
                            pixel_pos.y as i32,
                            texture_sample_map,
                            scale as usize,
                        );
                        let strength_sample = strength_sample_map[(x + y * size) as usize];
                        let wanted_color = texture_sample
                            .with_a(strength_sample * strength * time.delta_seconds() * 100.0);

                        let local_pixel_pos = master_terrain
                            .pixel_to_local_pixel_pos_with_chunk(pixel_pos, chunk_pos);
                        let pixel_index = (local_pixel_pos.x * 4
                            + local_pixel_pos.y * master_terrain.texture_size as u32 * 4)
                            as usize;

                        image_map
                            .get_mut(&chunk_pos)
                            .unwrap()
                            .as_mut()
                            .unwrap()
                            .push((wanted_color, pixel_index));
                        //master_terrain.draw(&mut image_map, pixel_pos, wanted_color);
                    }
                }

                for (chunk_pos, info) in image_map {
                    let pixels = if let Some(pixels) = info {
                        pixels
                    } else {
                        continue;
                    };
                    let handle =
                        if let Some(handle) = master_terrain.texture_map.textures.get(&chunk_pos) {
                            handle
                        } else {
                            return;
                        };
                    let image = if let Some(image) = images.get_mut(handle) {
                        image
                    } else {
                        return;
                    };
                    for (wanted_color, pixel_index) in pixels {
                        let current_color = Color::rgb_u8(
                            image.data[pixel_index + 0],
                            image.data[pixel_index + 1],
                            image.data[pixel_index + 2],
                        );
                        let calculated_color =
                            lerp_color(&current_color, &wanted_color, wanted_color.a()).with_a(1.0);
                        for (i, b) in calculated_color.as_rgba_u8().into_iter().enumerate() {
                            image.data[pixel_index + i] = b;
                        }
                    }
                }
            }
        }
    }
}
trait SampleTrait: Clone + Mul<f32, Output = Self> + Add + Sub {
    fn default() -> Self;
    fn mul_f32(self, val: f32) -> Self {
        self * val
    }
    fn add(self, other: Self) -> <Self as Add>::Output {
        self + other
    }
    fn sub(self, other: Self) -> <Self as Sub>::Output {
        self - other
    }
}
fn mod_neg(num: i32, mod_num: usize) -> usize {
    let m = num % (mod_num as i32);
    if m >= 0 {
        m as usize
    } else {
        (mod_num as i32 + m) as usize
    }
}
fn sample_repeating(x: i32, y: i32, values: &Vec<Color>, size: usize) -> Color {
    let x = mod_neg(x, size);
    let y = mod_neg(y, size);
    assert!(x < size);
    assert!(y < size);
    values[x + y * size]
}

fn lerp_color(col: &Color, other: &Color, mut t: f32) -> Color {
    t = t.clamp(0.0, 1.0);
    Color::rgba(
        col.r() + (other.r() - col.r()) * t,
        col.g() + (other.g() - col.g()) * t,
        col.b() + (other.b() - col.b()) * t,
        col.a() + (other.a() - col.a()) * t,
    )
}
