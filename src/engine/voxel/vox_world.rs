use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver},
    },
    thread::Thread,
};

use nalgebra::Vector3;
use paya::swapchain::{self, Swapchain};
use rayon::{spawn, ThreadBuilder};
use voxei_macros::Resource;

use crate::{
    constants,
    engine::{
        common::transform::Transform,
        ecs::ecs_world::ECSWorld,
        graphics::{
            device::DeviceResource, pass::voxel::VoxelPipeline, swapchain::SwapchainResource,
        },
        resource::{Res, ResMut},
        voxel::dynamic_world::SpatialStatus,
    },
    settings::Settings,
};

use super::{
    chunk_generator::ChunkGenerator,
    dynamic_world::DynVoxelWorld,
    util::{next_pow2, Morton},
    vox_constants::{CHUNK_LENGTH, CHUNK_WORLD_LENGTH},
};

#[derive(Resource)]
pub struct VoxelWorld {
    dyn_world: DynVoxelWorld,

    chunk_center: WorldChunkPos,
    last_gpu_requested_index: u64,

    chunk_render_distance: ChunkRadius,
    chunk_generator: ChunkGenerator,
}

impl VoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        Self {
            dyn_world: DynVoxelWorld::new(settings),

            chunk_center: WorldChunkPos::new(0, 0, 0),
            last_gpu_requested_index: 0,

            chunk_render_distance: settings.chunk_render_distance,
            chunk_generator: ChunkGenerator::new(),
        }
    }

    pub fn update_settings(mut vox_world: ResMut<VoxelWorld>, settings: Res<Settings>) {}

    pub fn update_world_streaming(
        mut vox_world: ResMut<VoxelWorld>,
        device: Res<DeviceResource>,
        swapchain: Res<SwapchainResource>,
        mut vox_pipeline: ResMut<VoxelPipeline>,
        settings: Res<Settings>,
    ) {
        let gpu_index = unsafe {
            device
                .handle()
                .get_semaphore_counter_value(swapchain.gpu_timeline_semaphore().handle())
                .unwrap()
        };

        let mut requested_bricks: HashSet<Morton> = HashSet::new();
        if gpu_index != vox_world.last_gpu_requested_index {
            for i in (((gpu_index as i64 - constants::MAX_FRAMES_IN_FLIGHT as i64 + 1)
                .max(vox_world.last_gpu_requested_index as i64)) as u64)
                ..=gpu_index
            {
                vox_pipeline.compile_brick_requests(&device, &mut requested_bricks, i, &settings);
            }
        }

        // Calculate chunks that should be loaded dynamically
        let mut dyn_load_queue = HashSet::new();

        let dyn_load_radius = Vector3::new(
            settings.chunk_dyn_loaded_distance.radius() as i32,
            settings.chunk_dyn_loaded_distance.radius() as i32,
            settings.chunk_dyn_loaded_distance.radius() as i32,
        );
        let dyn_chunk_min = vox_world.chunk_center.vector - dyn_load_radius;
        let dyn_chunk_max = vox_world.chunk_center.vector + dyn_load_radius;
        for x in dyn_chunk_min.x..dyn_chunk_max.x {
            for y in dyn_chunk_min.y..dyn_chunk_max.y {
                for z in dyn_chunk_min.z..dyn_chunk_max.z {
                    let world_pos = WorldChunkPos::new(x, y, z);
                    let Some(dyn_pos) = world_pos.to_dyn_pos(&vox_world) else {
                        println!("Uh oh something went wrong and we are exceeing the dynamic_world with our dynamic load radius which should never happen.");
                        continue;
                    };
                    if !vox_world.dyn_world.chunk_status(dyn_pos).is_loaded() {
                        vox_world.dyn_world.set_chunk_loading(dyn_pos);
                        dyn_load_queue.insert(world_pos);
                    }
                }
            }
        }

        // Every chunk in the queue is unloaded
        for chunk_pos in dyn_load_queue.iter() {
            // If dyn chunk pos doesn't exist in the StaticWorld, we need to generate it
            // pretend svo doesnt exist for now
            vox_world.chunk_generator.generate_chunk(*chunk_pos);
        }

        for chunk in vox_world.chunk_generator.collect_generated_chunks() {
            if dyn_load_queue.contains(&chunk.chunk_position) {
                dyn_load_queue.remove(&chunk.chunk_position);
                let Some(dyn_pos) = chunk.chunk_position.to_dyn_pos(&vox_world) else {
                    println!("Uh oh something went wrong and we are exceeing the dynamic_world with our dynamic load radius which should never happen.");
                    continue;
                };
                vox_world.dyn_world.set_generated_chunk(dyn_pos, chunk);
            }
        }
        // Process any generated chunks.

        //        for brick in requested_bricks {}
    }

    pub fn update_world_position(mut vox_world: ResMut<VoxelWorld>, ecs: Res<ECSWorld>) {
        let mut player_query = ecs.player_query::<&Transform>();
        let (_, transform) = player_query.player();
        let player_pos = transform.isometry.translation.vector;

        let chunk_center = WorldChunkPos::new(
            (player_pos.x / CHUNK_WORLD_LENGTH).floor() as i32,
            (player_pos.y / CHUNK_WORLD_LENGTH).floor() as i32,
            (player_pos.z / CHUNK_WORLD_LENGTH).floor() as i32,
        );

        if chunk_center != vox_world.chunk_center {
            println!("Chunk center changed: {:?}", chunk_center);
        }
    }

    fn load_brick(&mut self, world_brick_pos: Vector3<i32>) {}

    pub fn dyn_world(&self) -> &DynVoxelWorld {
        &self.dyn_world
    }

    pub fn dyn_world_mut(&mut self) -> &mut DynVoxelWorld {
        &mut self.dyn_world
    }

    pub fn chunk_center(&self) -> WorldChunkPos {
        self.chunk_center
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkRadius {
    radius: u32,
    pow2_side_length: u32,
}

impl ChunkRadius {
    pub fn new(radius: u32) -> Self {
        Self {
            radius,
            pow2_side_length: next_pow2(radius * 2),
        }
    }

    pub fn radius(&self) -> u32 {
        self.radius
    }

    pub fn pow2_side_length(&self) -> u32 {
        self.pow2_side_length
    }

    pub fn pow2_half_side_length(&self) -> u32 {
        self.pow2_side_length / 2
    }

    pub fn pow2_volume(&self) -> u64 {
        let side_length = self.pow2_side_length() as u64; // 64 = 1000000
        let side_length_power = side_length.trailing_zeros();
        1 << (side_length_power * 3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldChunkPos {
    pub vector: Vector3<i32>,
}

impl WorldChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            vector: Vector3::new(x, y, z),
        }
    }

    pub fn to_dyn_pos(&self, vox_world: &VoxelWorld) -> Option<DynChunkPos> {
        let slm = vox_world.chunk_render_distance.pow2_side_length() as i32 - 1;
        let hl = vox_world.chunk_render_distance.pow2_half_side_length() as i32;
        let local_pos = self.vector + Vector3::new(hl, hl, hl) + vox_world.chunk_center().vector;

        if local_pos.x < 0 || local_pos.y < 0 || local_pos.z < 0 {
            return None;
        }
        if local_pos.x >= slm || local_pos.y >= slm || local_pos.z >= slm {
            return None;
        }

        Some(DynChunkPos::new(
            local_pos.x as u32,
            local_pos.y as u32,
            local_pos.z as u32,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DynChunkPos {
    pub vector: Vector3<u32>,
}

impl DynChunkPos {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            vector: Vector3::new(x, y, z),
        }
    }

    pub fn to_dyn_brick_pos(&self) -> DynBrickPos {
        DynBrickPos::new(
            self.vector.x * CHUNK_LENGTH as u32,
            self.vector.y * CHUNK_LENGTH as u32,
            self.vector.z * CHUNK_LENGTH as u32,
        )
    }

    pub fn morton(&self) -> Morton {
        Morton::encode(self.vector)
    }
}

pub struct DynBrickPos {
    pub vector: Vector3<u32>,
}

impl DynBrickPos {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            vector: Vector3::new(x, y, z),
        }
    }

    pub fn morton(&self) -> Morton {
        Morton::encode(self.vector)
    }
}
