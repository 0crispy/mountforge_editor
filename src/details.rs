use bevy::prelude::*;
use bevy_mod_raycast::deferred::RaycastSource;
use rand::Rng;

use crate::{
    resize_vector,
    terrain::MasterTerrain,
    ui::{EditInfo, EditMode, UiHovered},
};
pub struct DetailsPlugin;
impl Plugin for DetailsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn, update_models));
    }
}

fn spawn(
    mut edit_info: ResMut<EditInfo>,
    raycast_source: Query<&RaycastSource<()>>,
    mut master_terrain: ResMut<MasterTerrain>,
    mouse: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    ui_hovered: Res<UiHovered>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !master_terrain.loaded {
        return;
    }
    if let EditMode::EditDetails = &edit_info.edit_mode {
        let raycast_source = raycast_source.single();
        for (_, intersection) in raycast_source.intersections() {
            if mouse.just_pressed(MouseButton::Left) && !ui_hovered.0 {
                let pos = intersection.position();
                let world_pos = master_terrain.vec2_to_world_pos(Vec2::new(pos.x, pos.z));
                if edit_info.details_info.brush_info.selected_brush.is_none() {
                    return;
                }
                let model_name = if edit_info.details_info.selected_detail_name.is_empty() {
                    return;
                } else {
                    edit_info.details_info.selected_detail_name.clone()
                };
                let size = edit_info.sculpt_info.brush_info.size;
                if edit_info
                    .details_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map_size
                    != size
                {
                    let brush = edit_info
                        .details_info
                        .brush_info
                        .selected_brush
                        .as_mut()
                        .unwrap();
                    brush.sample_map =
                        resize_vector(&brush.map, brush.map_size as usize, size as usize);
                    brush.sample_map_size = size;
                }
                let sample_map = &edit_info
                    .details_info
                    .brush_info
                    .selected_brush
                    .as_ref()
                    .unwrap()
                    .sample_map;
                let strength = edit_info.details_info.brush_info.strength;

                let mut rng = rand::thread_rng();
                for x in 0..size {
                    for y in 0..size {
                        let x_f32 = x as f32 - size as f32 * 0.5;
                        let y_f32 = y as f32 - size as f32 * 0.5;
                        let world_pos =
                            world_pos + master_terrain.vec2_to_world_pos(Vec2::new(x_f32, y_f32));
                        let chunk_pos = master_terrain.world_to_chunk_pos(world_pos);
                        if !master_terrain.does_chunk_exist(&chunk_pos) {
                            continue;
                        }
                        let local_pos = master_terrain.world_to_local_pos(world_pos);

                        let translation = Vec3::new(world_pos.x as f32, 0.0, world_pos.y as f32);

                        let brush_sample = sample_map[(x + y * size) as usize];
                        let chance = brush_sample * strength * 0.1;
                        let random_number: f32 = rng.gen();

                        if master_terrain.details.contains_key(&world_pos) {
                            if keys.pressed(KeyCode::ControlLeft) {
                                if random_number < chance * 10.0 {
                                    commands
                                        .entity(master_terrain.details[&world_pos])
                                        .despawn_recursive();
                                    master_terrain.details.remove(&world_pos);
                                }
                            }
                            continue;
                        }
                        if keys.pressed(KeyCode::ControlLeft) {
                            continue;
                        }

                        if random_number < chance {
                            let id = commands
                                .spawn((
                                    SceneBundle {
                                        scene: asset_server
                                            .load(format!("models/{}#Scene0", model_name)),
                                        transform: Transform::from_translation(translation)
                                            .with_scale(Vec3::ONE * 0.05),
                                        ..Default::default()
                                    },
                                    DetailModel {
                                        name: model_name.clone(),
                                        chunk_pos,
                                        local_pos,
                                    },
                                ))
                                .id();
                            master_terrain.details.insert(world_pos, id);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct DetailModel {
    pub name: String,
    pub chunk_pos: IVec2,
    pub local_pos: UVec2,
}

fn update_models(
    master_terrain: Res<MasterTerrain>,
    mut models: Query<(&mut Transform, &DetailModel)>,
) {
    for (mut model_tf, detail_model) in &mut models {
        let height =
            master_terrain.get_local_height(detail_model.local_pos, detail_model.chunk_pos);
        model_tf.translation.y = height;
    }
}
