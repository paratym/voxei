use std::collections::HashMap;

use nalgebra::Vector3;
use voxei_macros::Resource;

use crate::{
    constants::{self, CHUNK_LENGTH},
    engine::resource::{Res, ResMut},
    settings::Settings,
};

#[derive(Resource)]
pub struct VoxelWorld {
    bricks: Vec<Brick>,
    chunks: Vec<Chunk>,

    chunk_occupancy_grid: Vec<u8>,
    brick_indices_grid: Vec<BrickIndex>,

    current_chunk_render_distance: u32,
    render_distance_changed: bool,

    chunk_center: Vector3<i32>,
}

impl VoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        let mut s = Self {
            bricks: Vec::new(),
            chunks: Vec::new(),

            chunk_occupancy_grid: vec![0; (settings.chunk_render_distance * 2).pow(3) as usize],
            brick_indices_grid: vec![
                BrickIndex::new(BrickStatus::Unloaded, 0);
                (settings.chunk_render_distance as u64 * 2 * CHUNK_LENGTH).pow(3)
                    as usize
            ],

            current_chunk_render_distance: settings.chunk_render_distance,
            render_distance_changed: false,
            chunk_center: Vector3::new(1, 0, 0),
        };

        s.generate_chunk(Vector3::new(3, 3, 0));

        s
    }

    pub fn generate_chunk(&mut self, chunk_position: Vector3<i32>) {
        let translated_center = chunk_position - self.chunk_center;
        let translated_corner = (translated_center
            + Vector3::new(
                self.chunk_render_distance() as i32,
                self.chunk_render_distance() as i32,
                self.chunk_render_distance() as i32,
            ))
        .map(|a| a as u32);
        println!("Translated Corner: {:?}", translated_corner);
        let chunk_morton = morton_encode_uvec3(
            translated_corner.x,
            translated_corner.y,
            translated_corner.z,
            self.chunk_render_distance() * 2,
        );
        self.chunk_occupancy_grid[(chunk_morton as usize) >> 3] |= 1 << (chunk_morton & 0b111);
        println!("Chunk Morton: {}", chunk_morton);
    }

    pub fn update_settings(mut vox_world: ResMut<VoxelWorld>, settings: Res<Settings>) {
        if settings.chunk_render_distance != vox_world.current_chunk_render_distance {
            todo!("Change chunk render distance");
        }
    }

    pub fn chunk_occupancy_grid(&self) -> &[u8] {
        &self.chunk_occupancy_grid
    }

    pub fn chunk_render_distance(&self) -> u32 {
        self.current_chunk_render_distance
    }

    pub fn render_distance_changed(&self) -> bool {
        self.render_distance_changed
    }

    pub fn chunk_center(&self) -> Vector3<i32> {
        self.chunk_center
    }
}

#[repr(C)]
pub struct Chunk {}

#[derive(Debug, Clone, Copy)]
pub enum BrickStatus {
    Unloaded = 0b00,
    Loading = 0b01,
    Loaded = 0b10,
}

// 31 - 32, Brick Status, 00 - Unloaded, 01 - Loading, 10 - Loaded
// 30 - 0, Brick index
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BrickIndex(u32);

impl BrickIndex {
    pub fn new(status: BrickStatus, index: u32) -> Self {
        Self(((status as u32) << 30) | index)
    }

    pub fn status(&self) -> u32 {
        self.0 >> 30
    }

    pub fn index(&self) -> u32 {
        self.0 & 0x3FFFFFFF
    }
}

#[repr(C)]
pub struct Brick {}

fn morton_encode_uvec3(x: u32, y: u32, z: u32, side_length: u32) -> u32 {
    let height = (side_length as f32).log2() as u32;
    let mut answer = 0;
    for i in 0..=height {
        answer |= (x & (1 << i)) << (2 * i);
        answer |= (y & (1 << i)) << (2 * i + 1);
        answer |= (z & (1 << i)) << (2 * i + 2);
    }
    answer
}

pub struct Voxel {}
