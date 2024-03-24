use std::collections::HashMap;

use nalgebra::Vector3;
use voxei_macros::Resource;

use crate::engine::resource::ResMut;

use super::{
    octree::{ChunkOctree, VoxelOctree, NULL_INDEX},
    VoxelData,
};

pub const CHUNK_OCTREE_HEIGHT: u32 = 6;
pub const CHUNK_LENGTH: i32 = 1 << CHUNK_OCTREE_HEIGHT;

const LOADED_CHUNKS_LENGTH: i32 = 8;
const LOADED_CHUNK_VOXEL_LENGTH: i32 = CHUNK_LENGTH * LOADED_CHUNKS_LENGTH;

#[derive(Resource)]
pub struct VoxelWorld {
    chunk_tree: ChunkOctree,
    chunk_data: Vec<VoxelOctree>,
    voxel_data_lut: HashMap<VoxelData, u32>,
    voxel_data: Vec<VoxelData>,
    center: ChunkPosition,

    tree_updated: bool,
    chunks_updated: Vec<u32>,
    voxel_data_updated: bool,
}

impl VoxelWorld {
    pub fn new() -> Self {
        let mut s = Self {
            chunk_tree: ChunkOctree::new(LOADED_CHUNKS_LENGTH as u32),
            chunk_data: Vec::new(),
            voxel_data_lut: HashMap::new(),
            voxel_data: Vec::new(),
            center: ChunkPosition::new(0, 0, 0),

            tree_updated: false,
            chunks_updated: Vec::new(),
            voxel_data_updated: false,
        };

        s.add_voxel(
            VoxelPosition::new(0, 0, 0),
            VoxelData {
                color: (1.0, 0.6, 0.4),
            },
        );

        s
    }

    fn add_voxel(&mut self, position: VoxelPosition, data: VoxelData) {
        let position = position.translate_chunk(self.center);

        let chunk_morton = position.chunk_position().morton(self.chunk_tree.height());
        let voxel_morton = position.local_morton();
        println!("{:?}", chunk_morton);

        let voxel_data_index = self.get_or_insert_voxel_data(data);
        let chunk_data_index = self.chunk_tree.get_chunk(chunk_morton);

        if chunk_data_index == NULL_INDEX {
            let new_index = self.chunk_data.len() as u32;
            let mut new_chunk = VoxelOctree::new();
            new_chunk.add_voxel(voxel_morton, voxel_data_index);
            self.chunk_data.push(new_chunk);
            self.chunk_tree.create_chunk(chunk_morton, new_index);
            self.tree_updated = true;
            self.chunks_updated.push(new_index);
        } else {
            self.chunk_data[chunk_data_index as usize].add_voxel(voxel_morton, voxel_data_index);
            self.chunks_updated.push(voxel_data_index);
        }
    }

    fn get_or_insert_voxel_data(&mut self, data: VoxelData) -> u32 {
        if let Some(index) = self.voxel_data_lut.get(&data) {
            *index
        } else {
            let index = self.voxel_data.len() as u32;
            self.voxel_data.push(data);
            self.voxel_data_lut.insert(data, index);
            self.voxel_data_updated = true;
            index
        }
    }

    pub fn clear_changes(mut voxel_world: ResMut<VoxelWorld>) {
        voxel_world.tree_updated = false;
        voxel_world.chunks_updated.clear();
        voxel_world.voxel_data_updated = false;
    }

    pub fn did_tree_update(&self) -> bool {
        self.tree_updated
    }

    pub fn did_voxel_data_update(&self) -> bool {
        self.voxel_data_updated
    }

    pub fn chunks_updated(&self) -> &[u32] {
        &self.chunks_updated
    }

    pub fn chunk_data(&self) -> &[VoxelOctree] {
        &self.chunk_data
    }

    pub fn chunk_tree(&self) -> &ChunkOctree {
        &self.chunk_tree
    }

    pub fn voxel_data(&self) -> &[VoxelData] {
        &self.voxel_data
    }
}

impl std::fmt::Debug for VoxelWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoxelWorld")
            .field("chunk_tree", &self.chunk_tree)
            .field("chunk_data", &self.chunk_data)
            .field("voxel_data_lut", &self.voxel_data_lut)
            .field("voxel_data", &self.voxel_data)
            .field("center", &self.center)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkPosition(Vector3<i32>);

impl ChunkPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        println!("chunk position: x: {}, y: {}, z: {}", x, y, z);
        Self(Vector3::new(x, y, z))
    }

    pub fn morton(&self, height: u32) -> u32 {
        morton_encode_ivec3(self.x, self.y, self.z, height)
    }
}

impl std::ops::Deref for ChunkPosition {
    type Target = Vector3<i32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct VoxelPosition(Vector3<i32>);

impl VoxelPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(Vector3::new(x, y, z))
    }

    pub fn translate_chunk(&self, chunk_position: ChunkPosition) -> VoxelPosition {
        VoxelPosition::new(
            self.x - (chunk_position.x * CHUNK_LENGTH),
            self.y - (chunk_position.y * CHUNK_LENGTH),
            self.z - (chunk_position.z * CHUNK_LENGTH),
        )
    }

    pub fn chunk_position(&self) -> ChunkPosition {
        ChunkPosition::new(
            self.x / CHUNK_LENGTH,
            self.y / CHUNK_LENGTH,
            self.z / CHUNK_LENGTH,
        )
    }

    pub fn local_morton(&self) -> u32 {
        let local = Vector3::new(
            self.x % CHUNK_LENGTH,
            self.y % CHUNK_LENGTH,
            self.z % CHUNK_LENGTH,
        );
        morton_encode_uvec3(
            local.x as u32,
            local.y as u32,
            local.z as u32,
            CHUNK_OCTREE_HEIGHT,
        )
    }
}

impl std::ops::Deref for VoxelPosition {
    type Target = Vector3<i32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn morton_encode_ivec3(x: i32, y: i32, z: i32, height: u32) -> u32 {
    println!("ix: {}, y: {}, z: {}, height: {}", x, y, z, height);
    morton_encode_uvec3(
        (x + (1 << (height - 1))) as u32,
        (y + (1 << (height - 1))) as u32,
        (z + (1 << (height - 1))) as u32,
        height,
    )
}

fn morton_encode_uvec3(x: u32, y: u32, z: u32, height: u32) -> u32 {
    println!("x: {}, y: {}, z: {}, height: {}", x, y, z, height);
    let mut answer = 0;
    for i in 0..height {
        answer |= (x & (1 << i)) << (2 * i);
        answer |= (y & (1 << i)) << (2 * i + 1);
        answer |= (z & (1 << i)) << (2 * i + 2);
    }
    answer
}
