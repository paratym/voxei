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
        input::{keyboard::Key, Input},
        resource::{Res, ResMut},
        voxel::{dynamic_world::SpatialStatus, vox_constants::BRICK_WORLD_LENGTH},
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

    last_search_bounds: (Vector3<i32>, Vector3<i32>),
}

impl VoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        let mut s = Self {
            dyn_world: DynVoxelWorld::new(settings),

            chunk_center: WorldChunkPos::new(0, 0, 0),
            last_gpu_requested_index: 0,

            chunk_render_distance: settings.chunk_render_distance,
            chunk_generator: ChunkGenerator::new(),

            last_search_bounds: (Vector3::new(0, 0, 0), Vector3::new(0, 0, 0)),
        };

        s.chunk_generator
            .update_bounds(s.chunk_center, s.chunk_render_distance);

        s
    }

    pub fn update_settings(mut vox_world: ResMut<VoxelWorld>, settings: Res<Settings>) {}

    pub fn update_world_streaming(
        mut vox_world: ResMut<VoxelWorld>,
        device: Res<DeviceResource>,
        swapchain: Res<SwapchainResource>,
        vox_pipeline: ResMut<VoxelPipeline>,
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
        let mut dyn_load_queue = Vec::new();

        let timing = std::time::Instant::now();
        let mut visited_chunks = HashSet::new();
        let r = settings.chunk_generation_distance.radius() as i32;
        let dyn_load_radius = Vector3::new(r, r, r);
        let dyn_chunk_min = vox_world.chunk_center.vector - dyn_load_radius;
        let dyn_chunk_max = vox_world.chunk_center.vector + dyn_load_radius;
        let new_search_bounds = (dyn_chunk_min, dyn_chunk_max);
        let old_search_bounds = vox_world.last_search_bounds;

        let mut voxel_fn = |x: i32, y: i32, z: i32| {
            let world_pos = WorldChunkPos::new(x, y, z);
            if visited_chunks.contains(&world_pos) {
                return;
            }
            visited_chunks.insert(world_pos);

            let Some(dyn_pos) = world_pos.to_dyn_pos(&vox_world) else {
                println!("Uh oh something went wrong and we are exceeing the dynamic_world with our dynamic load radius which should never happen.");
                return;
            };
            if vox_world.dyn_world.chunk_status(dyn_pos) == SpatialStatus::Unloaded {
                vox_world.dyn_world.set_chunk_loading(dyn_pos);
                dyn_load_queue.push(world_pos);
            }
        };
        for x in dyn_chunk_min.x..=dyn_chunk_max.x {
            for y in dyn_chunk_min.y..=dyn_chunk_max.y {
                for z in dyn_chunk_min.z..=dyn_chunk_max.z {
                    voxel_fn(x, y, z);
                }
            }
        }

        vox_world.last_search_bounds = new_search_bounds;
        // println!("Time to calculate dyn load queue: {:?}", timing.elapsed());

        // Every chunk in the queue is unloaded
        for chunk_pos in dyn_load_queue.iter() {
            // If dyn chunk pos doesn't exist in the StaticWorld, we need to generate it
            // pretend svo doesnt exist for now
            vox_world.chunk_generator.generate_chunk(*chunk_pos);
        }

        for chunk in vox_world.chunk_generator.collect_generated_chunks() {
            let Some(dyn_pos) = chunk.chunk_position.to_dyn_pos(&vox_world) else {
                continue;
            };
            vox_world.dyn_world.set_generated_chunk(dyn_pos, chunk);
        }
        // Process any generated chunks.

        //        for brick in requested_bricks {}
    }

    pub fn update_world_position(
        mut vox_world: ResMut<VoxelWorld>,
        ecs: Res<ECSWorld>,
        input: Res<Input>,
    ) {
        let mut player_query = ecs.player_query::<&Transform>();
        let (_, transform) = player_query.player();
        let player_pos = transform.isometry.translation.vector;

        let chunk_center = WorldChunkPos::new(
            (player_pos.x / CHUNK_WORLD_LENGTH).floor() as i32,
            (player_pos.y / CHUNK_WORLD_LENGTH).floor() as i32,
            (player_pos.z / CHUNK_WORLD_LENGTH).floor() as i32,
        );

        // let mut chunk_center = vox_world.chunk_center;

        // if input.is_key_pressed(Key::Right) {
        //     chunk_center.vector.x += 2;
        // } else if input.is_key_pressed(Key::Left) {
        //     chunk_center.vector.x -= 2;
        // } else if input.is_key_pressed(Key::Up) {
        //     chunk_center.vector.z += 1;
        // } else if input.is_key_pressed(Key::Down) {
        //     chunk_center.vector.z -= 1;
        // } else if input.is_key_pressed(Key::PageUp) {
        //     chunk_center.vector.y += 1;
        // } else if input.is_key_pressed(Key::PageDown) {
        //     chunk_center.vector.y -= 1;
        // }

        if chunk_center != vox_world.chunk_center {
            let translation = chunk_center.vector - vox_world.chunk_center.vector;
            let old_chunk_center = vox_world.chunk_center;
            vox_world
                .dyn_world
                .update_translation(translation, old_chunk_center);
            vox_world.chunk_center = chunk_center;

            let chunk_render_distance = vox_world.chunk_render_distance;
            vox_world
                .chunk_generator
                .update_bounds(chunk_center, chunk_render_distance);
        }

        if input.is_key_pressed(Key::T) {
            const NORM_RANGE: i32 = 3;
            for x in -NORM_RANGE..=NORM_RANGE {
                for y in -NORM_RANGE..=NORM_RANGE {
                    for z in -NORM_RANGE..=NORM_RANGE {
                        let chunk_center = WorldChunkPos::new(
                            vox_world.chunk_center.vector.x + x,
                            vox_world.chunk_center.vector.y + y,
                            vox_world.chunk_center.vector.z + z,
                        );

                        let dyn_pos = chunk_center.to_dyn_pos(&vox_world).unwrap();
                        let brick_center = Vector3::new(
                            ((player_pos.x / BRICK_WORLD_LENGTH).floor() as i32)
                                .rem_euclid(CHUNK_LENGTH as i32) as u32,
                            ((player_pos.y / BRICK_WORLD_LENGTH).floor() as i32)
                                .rem_euclid(CHUNK_LENGTH as i32) as u32,
                            ((player_pos.z / BRICK_WORLD_LENGTH).floor() as i32)
                                .rem_euclid(CHUNK_LENGTH as i32) as u32,
                        );
                        println!("Chunk: {:?} Brick: {:?}", dyn_pos, brick_center);
                        let brick_morton =
                            Morton::encode(dyn_pos.to_dyn_brick_pos().vector + brick_center);
                        println!(
                            "Chunk: {:?} Brick: {:?}, morton: {:?}",
                            dyn_pos, brick_center, brick_morton
                        );
                        vox_world.dyn_world_mut().update_chunk_normals(dyn_pos);
                    }
                }
            }
        }
    }

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
            pow2_side_length: next_pow2(radius * 2 + 1),
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
        let slm = vox_world.chunk_render_distance.pow2_side_length() as i32;
        let hl = vox_world.chunk_render_distance.pow2_half_side_length() as i32;
        let local_pos = self.vector + Vector3::new(hl, hl, hl) - vox_world.chunk_center().vector;

        if local_pos.x < 0
            || local_pos.y < 0
            || local_pos.z < 0
            || local_pos.x >= slm
            || local_pos.y >= slm
            || local_pos.z >= slm
        {
            return None;
        }

        let mem_local_pos = local_pos + vox_world.dyn_world().chunk_translation().map(|x| x as i32);

        Some(DynChunkPos::new(
            mem_local_pos.x.rem_euclid(slm) as u32,
            mem_local_pos.y.rem_euclid(slm) as u32,
            mem_local_pos.z.rem_euclid(slm) as u32,
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
