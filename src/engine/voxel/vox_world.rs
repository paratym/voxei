use std::collections::{HashMap, HashSet};

use nalgebra::Vector3;
use paya::swapchain::{self, Swapchain};
use voxei_macros::Resource;

use crate::{
    constants::{self, CHUNK_LENGTH},
    engine::{
        graphics::{
            device::DeviceResource, pass::voxel::VoxelPipeline, swapchain::SwapchainResource,
        },
        resource::{Res, ResMut},
    },
    settings::Settings,
};

#[derive(Resource)]
pub struct VoxelWorld {
    chunk_occupancy_grid: Vec<u8>,
    brick_indices_grid: Vec<BrickIndex>,
    brick_data: Vec<Brick>,

    current_chunk_render_distance: u32,
    render_distance_changed: bool,

    chunk_center: Vector3<i32>,

    last_gpu_requested_index: u64,
}

impl VoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        Self {
            chunk_occupancy_grid: vec![
                255;
                (settings.chunk_render_distance * 2).pow(3) as usize / 8
            ],
            brick_indices_grid: vec![
                BrickIndex::new_unloaded();
                (settings.chunk_render_distance as u64 * 2 * CHUNK_LENGTH).pow(3)
                    as usize
            ],
            brick_data: Vec::new(),

            current_chunk_render_distance: settings.chunk_render_distance,
            render_distance_changed: false,
            chunk_center: Vector3::new(0, 0, 0),

            last_gpu_requested_index: 0,
        }
    }

    pub fn load_brick(&mut self, morton: u64) -> BrickIndex {
        let local_brick_pos = morton_decode_uvec3(
            morton,
            self.chunk_render_distance() * 2 * CHUNK_LENGTH as u32,
        );
        let world_brick_pos = local_brick_pos.map(|a| a as i64)
            + self
                .chunk_center
                .map(|a| (a - self.chunk_render_distance() as i32) as i64 * CHUNK_LENGTH as i64);

        // Terrain gen kinda
        let mut voxels = [0 as u8; constants::BRICK_VOLUME as usize / 8];
        let mut is_empty = true;
        for x in 0..constants::BRICK_LENGTH {
            for y in 0..constants::BRICK_LENGTH {
                for z in 0..constants::BRICK_LENGTH {
                    let voxel_pos = Vector3::new(x as i64, y as i64, z as i64);

                    if voxel_pos.y == 0 {
                        let voxel_morton = morton_encode_uvec3(
                            voxel_pos.x as u64,
                            voxel_pos.y as u64,
                            voxel_pos.z as u64,
                            constants::BRICK_LENGTH as u32,
                        );
                        voxels[voxel_morton as usize / 8] |= 1 << (voxel_morton & 0b111);
                        is_empty = false;
                    }
                }
            }
        }

        if is_empty {
            return BrickIndex::new_loaded_empty();
        }

        let brick = Brick {
            voxel_mask: voxels,
            info: 0,
        };

        let index = self.brick_data.len();
        self.brick_data.push(brick);
        BrickIndex::new_loaded(index as u32)
    }

    pub fn update_settings(mut vox_world: ResMut<VoxelWorld>, settings: Res<Settings>) {
        if settings.chunk_render_distance != vox_world.current_chunk_render_distance {
            todo!("Change chunk render distance");
        }
    }

    pub fn load_requested_bricks(
        mut vox_world: ResMut<VoxelWorld>,
        swapchain: Res<SwapchainResource>,
        device: Res<DeviceResource>,
        vox_pipeline: Res<VoxelPipeline>,
    ) {
        let gpu_index = unsafe {
            device
                .handle()
                .get_semaphore_counter_value(swapchain.gpu_timeline_semaphore().handle())
                .unwrap()
        };
        if gpu_index == vox_world.last_gpu_requested_index {
            return;
        }

        let mut requested_bricks: HashSet<u32> = HashSet::new();
        for i in (((gpu_index as i64 - constants::MAX_FRAMES_IN_FLIGHT as i64 + 1)
            .max(vox_world.last_gpu_requested_index as i64)) as u64)..=gpu_index
        {
            let frame_index = i % constants::MAX_FRAMES_IN_FLIGHT as u64;
            let buffer = vox_pipeline.brick_request_list_stage_buffer(frame_index);
            let ptr = device.map_buffer_typed::<u32>(buffer);

            let size = unsafe { ptr.read() };
            let ptr = unsafe { ptr.add(1) };
            for j in 0..size {
                requested_bricks.insert(unsafe { ptr.add(j as usize).read() });
            }
        }

        for brick in requested_bricks {
            println!("Loading brick {}", brick);
            vox_world.brick_indices_grid[brick as usize] = vox_world.load_brick(brick as u64);
        }

        vox_world.last_gpu_requested_index = gpu_index;
    }

    pub fn chunk_occupancy_grid(&self) -> &[u8] {
        &self.chunk_occupancy_grid
    }

    pub fn brick_indices_grid(&self) -> &[BrickIndex] {
        &self.brick_indices_grid
    }

    pub fn brick_data(&self) -> &[Brick] {
        &self.brick_data
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

#[derive(Debug, Clone, Copy)]
pub enum BrickStatus {
    Unloaded = 0b00,
    Loading = 0b01,
    Loaded = 0b10,
    LoadedEmpty = 0b11,
}

// 31 - 32, Brick Status, 00 - Unloaded, 01 - Loading, 10 - Loaded
// 30 - 0, Brick index
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BrickIndex(u32);

impl BrickIndex {
    pub fn new_unloaded() -> Self {
        Self(0)
    }

    pub fn new_loading(r: u8, g: u8, b: u8) -> Self {
        Self(((BrickStatus::Loading as u32) << 30) | (r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    pub fn new_loaded_empty() -> Self {
        Self((BrickStatus::LoadedEmpty as u32) << 30)
    }

    pub fn new_loaded(index: u32) -> Self {
        Self((BrickStatus::Loaded as u32) << 30 | index)
    }

    pub fn status(&self) -> u32 {
        self.0 >> 30
    }

    pub fn index(&self) -> u32 {
        self.0 & 0x3FFFFFFF
    }
}

#[repr(C)]
pub struct Brick {
    voxel_mask: [u8; 64],
    info: u32,
}

fn morton_encode_uvec3(x: u64, y: u64, z: u64, side_length: u32) -> u64 {
    let height = (side_length as f32).log2() as u32;
    let mut answer = 0;
    for i in 0..=height {
        answer |= (x & (1 << i)) << (2 * i);
        answer |= (y & (1 << i)) << (2 * i + 1);
        answer |= (z & (1 << i)) << (2 * i + 2);
    }
    answer
}

fn morton_decode_uvec3(morton: u64, side_length: u32) -> Vector3<u64> {
    let height = (side_length as f32).log2() as u32;
    let mut x = 0;
    let mut y = 0;
    let mut z = 0;
    for i in 0..=height {
        x |= (morton & (1 << (3 * i))) >> (2 * i);
        y |= (morton & (1 << (3 * i + 1))) >> (2 * i + 1);
        z |= (morton & (1 << (3 * i + 2))) >> (2 * i + 2);
    }
    Vector3::new(x, y, z)
}
