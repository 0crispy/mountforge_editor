use std::{
    collections::HashMap,
    sync::Mutex,
};

use bevy::{
    prelude::*,
    render::{
        mesh::{self, VertexAttributeValues},
        primitives::Aabb,
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
        texture::{ImageSampler, ImageSamplerDescriptor},
    },
};
use bevy_mod_raycast::prelude::RaycastMesh;
use noise::{
    core::open_simplex::open_simplex_2d,
    permutationtable::PermutationTable,
};
use serde::{Deserialize, Serialize};

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MasterTerrain::unloaded())
            .add_systems(
                Update,
                (
                    destroy_terrain_chunks,
                    gen_terrain_chunks,
                    update_terrain_chunks,
                    lod_update,
                    cleanup,
                )
                    .chain(),
            );
    }
}
pub struct TextureMap {
    pub textures: HashMap<IVec2, Handle<Image>>,
    pub materials: HashMap<IVec2, Handle<StandardMaterial>>,
}
impl TextureMap {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
            materials: HashMap::new(),
        }
    }
}

pub struct Heightmap {
    pub heightmaps: Mutex<HashMap<IVec2, Vec<f32>>>,
}
impl Heightmap {
    fn new() -> Self {
        Self {
            heightmaps: Mutex::new(HashMap::new()),
        }
    }
}
#[derive(PartialEq)]
enum UpdateChunk {
    All,
    Points(Vec<UVec2>),
}

#[derive(Resource)]
pub struct MasterTerrain {
    pub loaded: bool,

    pub chunk_size: usize,
    pub texture_size: usize,

    pub heightmap: Heightmap,
    pub texture_map: TextureMap,
    pub lod: LOD,

    pub chunks: HashMap<IVec2, (Entity, HashMap<usize, Entity>)>,
    chunk_spawn_queue: Vec<IVec2>,
    chunk_destroy_queue: Vec<IVec2>,
    update_positions: Mutex<HashMap<IVec2, UpdateChunk>>,

    pub details: HashMap<IVec2, Entity>,

    delete_entities: Vec<Entity>,
}
impl MasterTerrain {
    fn unloaded() -> Self {
        Self {
            loaded: false,
            chunk_size: 0,
            texture_size: 0,

            heightmap: Heightmap::new(),
            texture_map: TextureMap::new(),
            lod: LOD { levels: Vec::new() },

            chunks: HashMap::new(),
            chunk_spawn_queue: Vec::new(),
            chunk_destroy_queue: Vec::new(),
            update_positions: Mutex::new(HashMap::new()),

            details: HashMap::new(),

            delete_entities: Vec::new(),
        }
    }
    pub fn reset(&mut self) {
        let mut delete_entities = Vec::new();
        for (ent, _) in self.chunks.values() {
            delete_entities.push(*ent);
        }
        for ent in self.details.values() {
            delete_entities.push(*ent);
        }
        *self = Self::unloaded();
        self.delete_entities = delete_entities;
    }
    pub fn init(&mut self, chunk_size: usize, texture_size: usize, lod: Vec<LODLevel>) {
        self.reset();
        self.loaded = true;

        self.chunk_size = chunk_size;
        self.texture_size = texture_size;
        self.lod = LOD { levels: lod };
        self.spawn_chunk(IVec2::ZERO);
    }
    pub fn pixels_per_tile(&self) -> usize {
        self.texture_size / self.chunk_size
    }
    pub fn does_chunk_exist(&self, pos: &IVec2) -> bool {
        self.chunks.get(pos).is_some()
    }
    pub fn get_chunk_entity(&self, pos: &IVec2) -> Option<Entity> {
        if let Some(entities) = self.chunks.get(pos) {
            Some(entities.0)
        } else {
            None
        }
    }
    pub fn get_chunk_mesh_entity(&self, pos: &IVec2, lod: usize) -> Option<Entity> {
        if let Some(entities) = self.chunks.get(pos) {
            Some(entities.1[&lod].clone())
        } else {
            None
        }
    }
    pub fn count_neighbors(&self, pos: &IVec2) -> usize {
        let mut output = 0;
        if self.does_chunk_exist(&(*pos + IVec2::Y)) {
            output += 1;
        }
        if self.does_chunk_exist(&(*pos + IVec2::NEG_Y)) {
            output += 1;
        }
        if self.does_chunk_exist(&(*pos + IVec2::X)) {
            output += 1;
        }
        if self.does_chunk_exist(&(*pos + IVec2::NEG_X)) {
            output += 1;
        }
        output
    }
    pub fn spawn_chunk(&mut self, pos: IVec2) {
        self.chunk_spawn_queue.push(pos);
    }
    pub fn destroy_chunk(&mut self, pos: IVec2) {
        self.chunk_destroy_queue.push(pos);
    }
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunk_pos_from_vec2(&self, pos: &Vec2) -> IVec2 {
        let pos_x = if pos.x > 0.0 {
            (pos.x / self.chunk_size as f32) as i32
        } else {
            -(-pos.x / self.chunk_size as f32).ceil() as i32
        };
        let pos_y = if pos.y > 0.0 {
            (pos.y / self.chunk_size as f32) as i32
        } else {
            -(-pos.y / self.chunk_size as f32).ceil() as i32
        };
        IVec2::new(pos_x, pos_y)
    }
    pub fn vec2_to_world_pos(&self, pos: Vec2) -> IVec2 {
        IVec2::new(pos.x.ceil() as i32, pos.y.ceil() as i32)
    }
    pub fn vec2_to_pixel_pos(&self, pos: Vec2) -> IVec2 {
        IVec2::new(
            (pos.x * self.pixels_per_tile() as f32).ceil() as i32,
            (pos.y * self.pixels_per_tile() as f32).ceil() as i32,
        )
    }
    pub fn world_to_chunk_pos(&self, world_pos: IVec2) -> IVec2 {
        IVec2::new(
            world_pos.x.div_euclid(self.chunk_size as i32),
            world_pos.y.div_euclid(self.chunk_size as i32),
        )
    }
    pub fn pixel_to_chunk_pos(&self, pixel_pos: IVec2) -> IVec2 {
        IVec2::new(
            pixel_pos.x.div_euclid(self.texture_size as i32),
            pixel_pos.y.div_euclid(self.texture_size as i32),
        )
    }
    pub fn pixel_to_local_pixel_pos_with_chunk(&self, pixel_pos: IVec2, chunk: IVec2) -> UVec2 {
        UVec2::new(
            (pixel_pos.x - chunk.x * self.texture_size as i32) as u32,
            (pixel_pos.y - chunk.y * self.texture_size as i32) as u32,
        )
    }
    pub fn world_to_local_pos(&self, world_pos: IVec2) -> UVec2 {
        let chunk = self.world_to_chunk_pos(world_pos);
        UVec2 {
            x: (world_pos.x - chunk.x * self.chunk_size as i32) as u32,
            y: (world_pos.y - chunk.y * self.chunk_size as i32) as u32,
        }
    }
    pub fn get_height(&self, world_pos: IVec2) -> f32 {
        let chunk_pos = self.world_to_chunk_pos(world_pos);
        let local_pos = self.world_to_local_pos(world_pos);
        self.get_local_height(local_pos, chunk_pos)
    }
    pub fn get_local_height(&self, local_pos: UVec2, chunk_pos: IVec2) -> f32 {
        if self
            .heightmap
            .heightmaps
            .lock()
            .unwrap()
            .get(&chunk_pos)
            .is_none()
        {
            let heights = self.gen_heights(chunk_pos);
            self.heightmap
                .heightmaps
                .lock()
                .unwrap()
                .insert(chunk_pos, heights);
        }
        self.heightmap.heightmaps.lock().unwrap()[&chunk_pos]
            [local_pos.x as usize + local_pos.y as usize * self.chunk_size]
    }
    fn gen_heights(&self, chunk_pos: IVec2) -> Vec<f32> {
        let mut heights = vec![0.0; self.chunk_size * self.chunk_size];

        let (offset_x, offset_y) = (
            chunk_pos.x as f64 * self.chunk_size as f64,
            chunk_pos.y as f64 * self.chunk_size as f64,
        );

        let hasher = PermutationTable::new(1);
        for y in 0..self.chunk_size {
            for x in 0..self.chunk_size {
                let height_index = x + y * self.chunk_size;
                heights[height_index] = open_simplex_2d(
                    [(x as f64 + offset_x) * 0.1, (y as f64 + offset_y) * 0.1],
                    &hasher,
                ) as f32
                    * 7.0; //builder.get_value(x, y) as f32 * 10.0;
            }
        }
        heights
    }
    fn get_normal(x: usize, y: usize, mesh_size: usize, vertices: &Vec<[f32; 3]>) -> [f32; 3] {
        let vertex_index = x + y * (mesh_size + 1);
        let get_normal = |mut b: IVec2, mut c: IVec2| -> Option<Vec3> {
            b += IVec2::new(x as i32, y as i32);
            c += IVec2::new(x as i32, y as i32);
            if b.x < 0
                || b.x >= (mesh_size + 1) as i32
                || c.x < 0
                || c.x >= (mesh_size + 1) as i32
                || b.y < 0
                || b.y >= (mesh_size + 1) as i32
                || c.y < 0
                || c.y >= (mesh_size + 1) as i32
            {
                //TODO(instead of returning none, return the normal from another chunk, if it's loaded)
                return None;
            }
            let (b, c) = (
                b.x + b.y * (mesh_size as i32 + 1),
                c.x + c.y * (mesh_size as i32 + 1),
            );
            let (v_a, v_b, v_c) = (
                vertices[vertex_index],
                vertices[b as usize],
                vertices[c as usize],
            );
            let (vec_a, vec_b, vec_c) = (
                Vec3::from_array(v_a),
                Vec3::from_array(v_b),
                Vec3::from_array(v_c),
            );
            Some(-(vec_b - vec_a).cross(vec_c - vec_a))
        };
        let mut all_normals = Vec::new();
        if let Some(n) = get_normal(IVec2::NEG_Y, IVec2::X) {
            all_normals.push(n);
        }
        if let Some(n) = get_normal(IVec2::X, IVec2::Y) {
            all_normals.push(n);
        }
        if let Some(n) = get_normal(IVec2::Y, IVec2::NEG_X) {
            all_normals.push(n);
        }
        if let Some(n) = get_normal(IVec2::NEG_X, IVec2::NEG_Y) {
            all_normals.push(n);
        }
        let normal: Vec3 = all_normals.iter().sum::<Vec3>() / all_normals.len() as f32;
        normal.to_array()
    }
    fn generate_mesh(&self, lod: usize) -> Mesh {
        let chunk_size = self.chunk_size;
        let mesh_size = self.mesh_size(lod);

        let vertex_count = (mesh_size + 1) * (mesh_size + 1);
        let mut vertices = vec![[0.0; 3]; vertex_count];
        let mut uvs = vec![[0.0; 2]; vertex_count];
        let mut triangles = vec![0; mesh_size * mesh_size * 6];
        let mut triangle_index = 0;
        let mut add_triangle = |t: [u32; 3]| {
            triangles[triangle_index] = t[2];
            triangles[triangle_index + 1] = t[1];
            triangles[triangle_index + 2] = t[0];
            triangle_index += 3;
        };
        for x in 0..=mesh_size {
            for y in 0..=mesh_size {
                let vertex_index = x + y * (mesh_size + 1);
                vertices[vertex_index] = [
                    x as f32 / mesh_size as f32 * chunk_size as f32,
                    0.0,
                    y as f32 / mesh_size as f32 * chunk_size as f32,
                ];
                if x < mesh_size && y < mesh_size {
                    add_triangle([
                        vertex_index as u32,
                        vertex_index as u32 + (mesh_size + 1) as u32 + 1,
                        vertex_index as u32 + (mesh_size + 1) as u32,
                    ]);
                    add_triangle([
                        vertex_index as u32 + (mesh_size + 1) as u32 + 1,
                        vertex_index as u32,
                        vertex_index as u32 + 1,
                    ]);
                }

                uvs[x + y * (mesh_size + 1)] = [
                    x as f32 / (mesh_size + 1) as f32,
                    y as f32 / (mesh_size + 1) as f32,
                ];
            }
        }

        let normals = vec![[0.0, 1.0, 0.0]; vertices.len()];
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(mesh::Indices::U32(triangles.to_vec())));
        mesh
    }
    fn gen_terrain_meshes(&self, chunk_pos: IVec2) -> Vec<(usize, Mesh)> {
        self.update_position_all(chunk_pos);
        let mut output = Vec::new();
        for lod in &self.lod.levels {
            output.push((lod.mesh_reduce, self.generate_mesh(lod.mesh_reduce)));
        }
        output
    }
    fn update_position_all(&self, chunk_pos: IVec2) {
        if self
            .update_positions
            .lock()
            .unwrap()
            .get(&chunk_pos)
            .is_none()
        {
            self.update_positions
                .lock()
                .unwrap()
                .insert(chunk_pos, UpdateChunk::All);
        }
    }
    fn update_position_add_point(&self, chunk_pos: IVec2, point: UVec2) {
        let mut update_positions = self.update_positions.lock().unwrap();
        match update_positions.get(&chunk_pos) {
            Some(update_chunk) => {
                if *update_chunk == UpdateChunk::All {
                    return;
                }
            }
            None => {
                update_positions.insert(chunk_pos, UpdateChunk::Points(Vec::new()));
            }
        }
        match update_positions.get_mut(&chunk_pos).unwrap() {
            UpdateChunk::Points(vertices) => {
                vertices.push(point);
            }
            _ => panic!(),
        }
    }
    pub fn set_height(&self, world_pos: IVec2, value: f32) {
        self.get_height(world_pos);

        let chunk_pos = self.world_to_chunk_pos(world_pos);
        let local_pos = self.world_to_local_pos(world_pos);
        self.heightmap
            .heightmaps
            .lock()
            .unwrap()
            .get_mut(&chunk_pos)
            .unwrap()[(local_pos.x + local_pos.y * self.chunk_size as u32) as usize] = value;
        self.update_position_all(chunk_pos);

        self.update_position_add_point(chunk_pos, local_pos);
        //self.update_position_add_point(chunk_pos + IVec2::NEG_Y, UVec2::new(local_pos.x,self.chunk_size as u32));

        if local_pos.x == 0 {
            self.update_position_add_point(
                chunk_pos + IVec2::NEG_X,
                UVec2::new(self.chunk_size as u32, local_pos.y),
            );
        }
        if local_pos.y == 0 {
            self.update_position_add_point(
                chunk_pos + IVec2::NEG_Y,
                UVec2::new(local_pos.x, self.chunk_size as u32),
            );
        }
        if local_pos.x == 0 && local_pos.y == 0 {
            self.update_position_add_point(
                chunk_pos + IVec2::NEG_ONE,
                UVec2::splat(self.chunk_size as u32),
            );
        }
    }
    pub fn add_height(&self, world_pos: IVec2, value: f32) {
        self.set_height(world_pos, self.get_height(world_pos) + value);
    }
    fn mesh_size(&self, lod: usize) -> usize {
        self.chunk_size / (2_u32.pow(lod as u32)) as usize
    }
}
#[derive(Component)]
pub struct Chunk {
    calculated_lod: usize,
}
#[derive(Component)]
pub struct ChunkMesh {
    lod: usize,
}
fn gen_terrain_chunks(
    mut master_terrain: ResMut<MasterTerrain>,
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let chunk_size = master_terrain.chunk_size;
    let texture_size = master_terrain.texture_size;

    for chunk_pos in master_terrain.chunk_spawn_queue.clone() {
        //Check if we aren't accidentaly spawning a chunk
        //that already exists
        if master_terrain.chunks.get(&chunk_pos).is_some() {
            continue;
        }
        //Texture generation
        let handle = {
            if let Some(handle) = master_terrain.texture_map.textures.get(&chunk_pos) {
                handle.clone()
            } else {
                let mut new_image = Image::new(
                    Extent3d {
                        width: texture_size as u32,
                        height: texture_size as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    vec![255; texture_size * texture_size * 4],
                    TextureFormat::Rgba8UnormSrgb,
                );
                new_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
                let handle = images.add(new_image);
                master_terrain
                    .texture_map
                    .textures
                    .insert(chunk_pos, handle.clone());
                handle.clone()
            }
        };
        let material = StandardMaterial {
            base_color_texture: Some(handle.clone()),
            perceptual_roughness: 0.8,
            reflectance: 0.02,
            ..Default::default()
        };
        master_terrain
            .texture_map
            .materials
            .insert(chunk_pos, materials.add(material));
        let weak_material_handle = master_terrain.texture_map.materials[&chunk_pos].clone_weak();

        let meshes = master_terrain.gen_terrain_meshes(chunk_pos);
        let mut mesh_entities = HashMap::new();
        let entity = commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(Vec3::new(
                        (chunk_pos.x as f32) * chunk_size as f32,
                        0.0,
                        (chunk_pos.y as f32) * chunk_size as f32,
                    )),
                    ..Default::default()
                },
                Chunk { calculated_lod: 0 },
            ))
            .with_children(|parent| {
                for (lod, mesh) in meshes {
                    let entity = parent
                        .spawn(PbrBundle {
                            mesh: mesh_assets.add(mesh),
                            material: weak_material_handle.clone(),
                            ..default()
                        })
                        //.insert(Wireframe)
                        .insert(RaycastMesh::<()>::default())
                        .insert(ChunkMesh { lod })
                        .id();
                    mesh_entities.insert(lod, entity);
                }
            })
            .id();
        master_terrain
            .chunks
            .insert(chunk_pos, (entity, mesh_entities));
        //let terrain_chunk = TerrainChunk { entity, heightmap };
    }
    master_terrain.chunk_spawn_queue.clear();
}

fn destroy_terrain_chunks(mut master_terrain: ResMut<MasterTerrain>, mut commands: Commands) {
    for chunk_pos in master_terrain.chunk_destroy_queue.clone() {
        if master_terrain.chunk_count() == 1 {
            //Don't delete the single remaining chunk,
            //as you won't be able to create any chunks :p
            break;
        }
        if let Some(chunk) = master_terrain.chunks.get(&chunk_pos) {
            commands.entity(chunk.0).despawn_recursive();
            master_terrain.chunks.remove(&chunk_pos);
        }
    }
    master_terrain.chunk_destroy_queue.clear();
}
fn update_terrain_chunks(
    master_terrain: Res<MasterTerrain>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    terrain_meshes: Query<&Handle<Mesh>>,
    mut commands: Commands,
    chunks: Query<&Chunk>,
) {
    let chunk_size = master_terrain.chunk_size;
    let mut chunk_mesh_entities = Vec::new();
    master_terrain
        .update_positions
        .lock()
        .unwrap()
        .retain(|chunk_pos, update_chunk| {
            if !master_terrain.chunks.contains_key(chunk_pos) {
                return false;
            }
            let mut mesh_entities = Vec::new();
            if let Some(chunk_ent) = master_terrain.get_chunk_entity(&chunk_pos) {
                if chunks.get(chunk_ent).is_ok() {
                    for lod in &master_terrain.lod.levels {
                        if let Some(chunk_mesh_ent) =
                            master_terrain.get_chunk_mesh_entity(&chunk_pos, lod.mesh_reduce)
                        {
                            chunk_mesh_entities.push(chunk_mesh_ent);
                            mesh_entities.push((lod.mesh_reduce, chunk_mesh_ent));
                        } else {
                            return true;
                        }
                    }
                } else {
                    return true;
                }
            } else {
                return true;
            };

            let update_points = match update_chunk {
                UpdateChunk::All => {
                    let mut update_points = Vec::new();
                    for y in 0..=chunk_size {
                        for x in 0..=chunk_size {
                            update_points.push(UVec2::new(x as u32, y as u32));
                        }
                    }
                    update_points
                }
                UpdateChunk::Points(vertices) => vertices.clone(),
            };
            for (lod, mesh_ent) in mesh_entities.into_iter() {
                let mesh_handle = if let Ok(h) = terrain_meshes.get(mesh_ent) {
                    h
                } else {
                    return true;
                };
                let my_mesh = mesh_assets.get_mut(mesh_handle).unwrap();

                let vertices = match my_mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap() {
                    VertexAttributeValues::Float32x3(vertices) => vertices,
                    _ => panic!(),
                };
                let mesh_size = master_terrain.mesh_size(lod);
                for UVec2 { x, y } in update_points.iter() {
                    if x % 2_u32.pow(lod as u32) != 0 || y % 2_u32.pow(lod as u32) != 0 {
                        continue;
                    }
                    let vert_x = ((*x as f32 / chunk_size as f32 * mesh_size as f32) as usize)
                        .clamp(0, mesh_size);
                    let vert_y = ((*y as f32 / chunk_size as f32 * mesh_size as f32) as usize)
                        .clamp(0, mesh_size);
                    let vertice_index = vert_x + vert_y * (mesh_size + 1);
                    let world_pos = IVec2::new(*x as i32, *y as i32)
                        + IVec2::splat(chunk_size as i32) * *chunk_pos;
                    let vertice_height = master_terrain.get_height(world_pos);
                    vertices[vertice_index][1] = vertice_height;
                }
                let vertices = match my_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
                    VertexAttributeValues::Float32x3(vertices) => vertices,
                    _ => panic!(),
                };
                let mut update_normals = HashMap::new();
                for UVec2 { x, y } in update_points.iter() {
                    if x % 2_u32.pow(lod as u32) != 0 || y % 2_u32.pow(lod as u32) != 0 {
                        continue;
                    }
                    let vert_x = ((*x as f32 / chunk_size as f32 * mesh_size as f32) as usize)
                        .clamp(0, mesh_size);
                    let vert_y = ((*y as f32 / chunk_size as f32 * mesh_size as f32) as usize)
                        .clamp(0, mesh_size);

                    let vertice_index = vert_x + vert_y * (mesh_size + 1);
                    update_normals.insert(
                        vertice_index,
                        MasterTerrain::get_normal(
                            vert_x as usize,
                            vert_y as usize,
                            mesh_size,
                            vertices,
                        ),
                    );
                }
                let normals = match my_mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL).unwrap() {
                    VertexAttributeValues::Float32x3(normals) => normals,
                    _ => panic!(),
                };
                for (vertice_index, normal) in update_normals {
                    normals[vertice_index] = normal;
                }
            }
            false
        });
    for ent in chunk_mesh_entities {
        commands.entity(ent).remove::<Aabb>();
    }
}
#[derive(Deserialize, Serialize, Clone)]
pub struct LODLevel {
    mesh_reduce: usize,
    max_view_distance: f32,
}
impl LODLevel {
    pub fn new(mesh_reduce: usize, max_view_distance: f32) -> Self {
        Self {
            mesh_reduce,
            max_view_distance,
        }
    }
}
pub struct LOD {
    pub levels: Vec<LODLevel>,
}
impl LOD {
    pub fn get(&self, distance: f32) -> usize {
        for lod_level in &self.levels {
            if distance < lod_level.max_view_distance {
                return lod_level.mesh_reduce;
            }
        }
        panic!();
    }
}
fn lod_update(
    master_terrain: Res<MasterTerrain>,
    mut mesh_parents: Query<(&Children, &mut Chunk)>,
    mut meshes: Query<(
        &GlobalTransform,
        &mut Visibility,
        &ChunkMesh,
        &Aabb,
        &Handle<StandardMaterial>,
    )>,
    camera: Query<&Transform, With<Camera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) {
    let camera_tf = camera.single();
    for (mesh_parent, mut chunk) in &mut mesh_parents {
        for (i, mesh_child) in mesh_parent.iter().enumerate() {
            if let Ok((chunk_mesh_tf, mut visibility, chunk_mesh, aabb, m)) =
                meshes.get_mut(*mesh_child)
            {
                if let Some(m) = materials.get_mut(m) {
                    if let Some(_) = images.get(m.base_color_texture.as_ref().unwrap()) {}
                }
                if i == 0 {
                    let distance = aabb_distance_to_point(
                        chunk_mesh_tf.translation(),
                        aabb,
                        camera_tf.translation,
                    );
                    let lod = master_terrain.lod.get(distance);
                    chunk.calculated_lod = lod;
                }
                *visibility = if chunk_mesh.lod == chunk.calculated_lod {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

pub fn aabb_distance_to_point(aabb_pos: Vec3, aabb: &Aabb, point: Vec3) -> f32 {
    let mut distance_squared = 0.0;
    // For each axis, calculate the distance to the box
    for i in 0..3 {
        let v = (point[i] - (aabb.center[i] + aabb_pos[i])).abs() - aabb.half_extents[i];
        distance_squared += v.max(0.0) * v.max(0.0);
    }
    distance_squared.sqrt()
}

fn cleanup(mut master_terrain: ResMut<MasterTerrain>, mut commands: Commands) {
    for ent in &master_terrain.delete_entities {
        commands.entity(*ent).despawn_recursive();
    }
    master_terrain.delete_entities.clear();
}