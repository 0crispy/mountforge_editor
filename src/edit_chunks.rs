use bevy::{pbr::NotShadowCaster, prelude::*};

use crate::{
    terrain::{ChunkMesh, MasterTerrain},
    ui::{EditChunksAction, EditInfo, EditMode, UiHovered},
};

pub struct EditChunksPlugin;
impl Plugin for EditChunksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_chunks);
    }
}

#[derive(Component)]
struct EditChunkGraphic;

fn edit_chunks(
    mut edit_info: ResMut<EditInfo>,
    camera_tf: Query<(&GlobalTransform, &Camera)>,
    mut master_terrain: ResMut<MasterTerrain>,
    window: Query<&Window>,
    mut edit_chunk_graphic: Query<(&mut Transform, Entity), With<EditChunkGraphic>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    buttons: Res<Input<MouseButton>>,
    chunk_materials: Query<&Handle<StandardMaterial>, With<ChunkMesh>>,
    ui_hovered: Res<UiHovered>,
) {
    if !master_terrain.loaded {
        return;
    }
    if let EditMode::EditChunks = &edit_info.edit_mode {
        let mut selected_chunk = None;
        //find intersection point of z=0 plane
        let (camera_gtf, camera) = camera_tf.single();

        if let Ok(window) = window.get_single() {
            if let Some(cursor) = window.cursor_position() {
                if let Some(camera_ray) = camera.viewport_to_world(camera_gtf, cursor) {
                    let t = -camera_ray.origin.y / camera_ray.direction.y;

                    if t > 0.0 {
                        //the camera ray intersect the z=0 plane!
                        let intersection_point = camera_ray.origin + camera_ray.direction * t;

                        let chunk_position = master_terrain.chunk_pos_from_vec2(&Vec2::new(
                            intersection_point.x,
                            intersection_point.z,
                        ));
                        selected_chunk = Some(chunk_position);
                    }
                }
            }
        }
        if selected_chunk.is_none() {
            if let Ok((_, add_chunk_graphic_entity)) = edit_chunk_graphic.get_single() {
                commands
                    .entity(add_chunk_graphic_entity)
                    .despawn_recursive();
            }
            return;
        }
        let selected_chunk = selected_chunk.unwrap();
        match edit_info.edit_chunks_info.action_type {
            EditChunksAction::Add => {
                if !master_terrain.does_chunk_exist(&selected_chunk)
                    && master_terrain.count_neighbors(&selected_chunk) > 0
                {
                    let graphic_position = (selected_chunk * master_terrain.chunk_size as i32)
                        .as_vec2()
                        + Vec2::ONE * master_terrain.chunk_size as f32 * 0.5;
                    let add_chunk_graphic_translation =
                        Vec3::new(graphic_position.x, 0.0, graphic_position.y);
                    if let Ok((mut edit_chunk_graphic_tf, _)) = edit_chunk_graphic.get_single_mut()
                    {
                        edit_chunk_graphic_tf.translation = add_chunk_graphic_translation;
                    } else {
                        commands.spawn((
                            PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Plane::default())),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::LIME_GREEN.with_a(0.5),
                                    unlit: true,
                                    double_sided: true,
                                    cull_mode: None,
                                    alpha_mode: AlphaMode::Add,
                                    ..Default::default()
                                }),
                                transform: Transform::from_scale(
                                    master_terrain.chunk_size as f32 * Vec3::ONE,
                                )
                                .with_translation(add_chunk_graphic_translation),
                                ..default()
                            },
                            NotShadowCaster,
                            Name::new("Edit chunk graphic"),
                            EditChunkGraphic,
                        ));
                    }

                    if buttons.pressed(MouseButton::Left) && !ui_hovered.0 {
                        master_terrain.spawn_chunk(selected_chunk);
                    }
                } else {
                    if let Ok((_, edit_chunk_graphic_entity)) = edit_chunk_graphic.get_single() {
                        commands
                            .entity(edit_chunk_graphic_entity)
                            .despawn_recursive();
                    }
                }

                if let Some(red_chunk) = edit_info.edit_chunks_info.red_chunk {
                    edit_info.edit_chunks_info.red_chunk = None;
                    if let Some(red_chunk_ent) = master_terrain.get_chunk_entity(&red_chunk) {
                        if let Ok(chunk_mat) = chunk_materials.get(red_chunk_ent) {
                            if let Some(mat) = materials.get_mut(chunk_mat) {
                                mat.base_color = Color::WHITE;
                            }
                        }
                    }
                }
            }
            EditChunksAction::Remove => {
                if let Some(selected_chunk_ent) = master_terrain.get_chunk_entity(&selected_chunk) {
                    if master_terrain.chunk_count() > 1 {
                        if let Some(red_chunk) = edit_info.edit_chunks_info.red_chunk {
                            if red_chunk != selected_chunk {
                                if let Some(red_chunk_ent) =
                                    master_terrain.get_chunk_entity(&red_chunk)
                                {
                                    if let Ok(chunk_mat) = chunk_materials.get(red_chunk_ent) {
                                        if let Some(mat) = materials.get_mut(chunk_mat) {
                                            mat.base_color = Color::WHITE;
                                        }
                                    }
                                }
                                edit_info.edit_chunks_info.red_chunk = Some(selected_chunk);
                                if let Ok(chunk_mat) = chunk_materials.get(selected_chunk_ent) {
                                    if let Some(mat) = materials.get_mut(chunk_mat) {
                                        mat.base_color = Color::RED;
                                    }
                                }
                            }
                        } else {
                            edit_info.edit_chunks_info.red_chunk = Some(selected_chunk);
                            if let Ok(chunk_mat) = chunk_materials.get(selected_chunk_ent) {
                                if let Some(mat) = materials.get_mut(chunk_mat) {
                                    mat.base_color = Color::RED;
                                }
                            }
                        }
                        if buttons.pressed(MouseButton::Left) && !ui_hovered.0 {
                            edit_info.edit_chunks_info.red_chunk = None;
                            master_terrain.destroy_chunk(selected_chunk);
                        }
                    }
                } else {
                    if let Some(red_chunk) = edit_info.edit_chunks_info.red_chunk {
                        edit_info.edit_chunks_info.red_chunk = None;
                        if let Some(red_chunk_ent) = master_terrain.get_chunk_entity(&red_chunk) {
                            if let Ok(chunk_mat) = chunk_materials.get(red_chunk_ent) {
                                if let Some(mat) = materials.get_mut(chunk_mat) {
                                    mat.base_color = Color::WHITE;
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        if let Ok((_, add_chunk_graphic_entity)) = edit_chunk_graphic.get_single() {
            commands
                .entity(add_chunk_graphic_entity)
                .despawn_recursive();
        }
    }
}
