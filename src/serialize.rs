use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{ImageSampler, ImageSamplerDescriptor},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    details::DetailModel,
    terrain::{LODLevel, MasterTerrain, LOD},
};

pub struct SerializePlugin;
impl Plugin for SerializePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Serializer::new())
            .add_systems(Update, (serialize, deserialize));
    }
}

#[derive(Resource)]
pub struct Serializer {
    serialize_path: Option<PathBuf>,
    deserialize_path: Option<PathBuf>,
}
impl Serializer {
    pub fn new() -> Self {
        Self {
            serialize_path: None,
            deserialize_path: None,
        }
    }
    pub fn serialize(&mut self, path: PathBuf) {
        self.serialize_path = Some(path);
    }
    pub fn deserialize(&mut self, path: PathBuf) {
        self.deserialize_path = Some(path);
    }
}
#[derive(Serialize, Deserialize)]
pub struct TerrainData {
    pub chunk_size: usize,
    pub texture_size: usize,
    pub lod: Vec<LODLevel>,

    pub chunks: Vec<ChunkData>,

    pub details: Vec<DetailData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub pos: IVec2,
    pub heights: Vec<f32>,
    pub texture: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
pub struct DetailData {
    pub name: String,
    pub chunk_pos: IVec2,
    pub local_pos: UVec2,
}

fn serialize(
    mut serializer: ResMut<Serializer>,
    master_terrain: Res<MasterTerrain>,
    images: Res<Assets<Image>>,
    detail_models: Query<&DetailModel>,
) {
    if let Some(path) = &serializer.serialize_path {
        let mut chunks = HashMap::new();
        for chunk_pos in master_terrain.chunks.keys() {
            chunks.insert(
                *chunk_pos,
                ChunkData {
                    pos: *chunk_pos,
                    heights: Vec::new(),
                    texture: Vec::new(),
                },
            );
        }
        for (chunk_pos, heights) in master_terrain.heightmap.heightmaps.lock().unwrap().iter() {
            if let Some(data) = chunks.get_mut(chunk_pos) {
                data.heights = heights.clone();
            }
        }
        for (chunk_pos, image_handle) in master_terrain.texture_map.textures.iter() {
            if let Some(data) = chunks.get_mut(chunk_pos) {
                if let Some(image) = images.get(image_handle) {
                    data.texture = image.data.clone();
                }
            }
        }
        let chunks = chunks.values().cloned().collect();
        let mut details = Vec::new();
        for detail_model in &detail_models {
            details.push(DetailData {
                name: detail_model.name.clone(),
                chunk_pos: detail_model.chunk_pos,
                local_pos: detail_model.local_pos,
            })
        }
        let data = TerrainData {
            chunk_size: master_terrain.chunk_size,
            texture_size: master_terrain.texture_size,
            lod: master_terrain.lod.levels.clone(),
            chunks,
            details,
        };
        // Serialize the data to binary using bincode
        let serialized = bincode::serialize(&data).expect("Serialization failed");

        // Write the serialized data to a file
        let final_path = path.with_extension("mf");
        let mut file = File::create(final_path).expect("Failed to create file");
        file.write_all(&serialized)
            .expect("Failed to write to file");
    }
    serializer.serialize_path = None;
}
fn deserialize(
    mut serializer: ResMut<Serializer>,
    mut master_terrain: ResMut<MasterTerrain>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Some(path) = &serializer.deserialize_path {
        let mut file = File::open(path).expect("Failed to open file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read from file");

        // Deserialize the data back into your data structure
        let data: TerrainData = bincode::deserialize(&buffer).expect("Deserialization failed");

        master_terrain.reset();
        master_terrain.chunk_size = data.chunk_size;
        master_terrain.texture_size = data.texture_size;
        master_terrain.lod = LOD { levels: data.lod };

        for chunk_data in data.chunks {
            master_terrain.spawn_chunk(chunk_data.pos);
            master_terrain
                .heightmap
                .heightmaps
                .lock()
                .unwrap()
                .insert(chunk_data.pos, chunk_data.heights);
            let mut new_image = Image::new(
                Extent3d {
                    width: data.texture_size as u32,
                    height: data.texture_size as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                chunk_data.texture,
                TextureFormat::Rgba8UnormSrgb,
            );
            new_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
            let handle = images.add(new_image);
            master_terrain
                .texture_map
                .textures
                .insert(chunk_data.pos, handle.clone());
        }
        for detail in data.details {
            let world_pos = detail.local_pos.as_ivec2() + detail.chunk_pos * data.chunk_size as i32;
            let translation = Vec3::new(world_pos.x as f32, 0.0, world_pos.y as f32);
            let id = commands
                .spawn((
                    SceneBundle {
                        scene: asset_server.load(format!("models/{}#Scene0", detail.name)),
                        transform: Transform::from_translation(translation)
                            .with_scale(Vec3::ONE * 0.05),
                        ..Default::default()
                    },
                    DetailModel {
                        name: detail.name,
                        chunk_pos: detail.chunk_pos,
                        local_pos: detail.local_pos,
                    },
                ))
                .id();
            master_terrain.details.insert(world_pos, id);
        }
        master_terrain.loaded = true;
    }
    serializer.deserialize_path = None;
}
