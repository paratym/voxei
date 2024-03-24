use paya::{
    allocator::MemoryFlags,
    command_recorder::{self, CommandRecorder},
    common::{AccessFlags, BufferTransition, BufferUsageFlags},
    device::Device,
    gpu_resources::{BufferId, BufferInfo, ImageId, PackedGpuResourceId},
};
use voxei_macros::Resource;

use crate::engine::{
    assets::{
        asset::Assets,
        watched_shaders::{ShaderDependencySignal, WatchedShaders},
    },
    graphics::pipeline_manager::{self, PipelineId, PipelineManager},
    voxel::{
        octree::{ChunkOctreeNode, VoxelOctreeNode},
        vox_world::{self, VoxelWorld},
        VoxelData,
    },
};

const RAY_MARCH_PATH: &str = "shaders/ray_march.comp.glsl";

const CHUNK_TREE_INITIAL_SIZE: u64 = 1024;
const CHUNK_TREE_ELEMENT_SIZE: u64 = std::mem::size_of::<ChunkOctreeNode>() as u64;
const CHUNK_DATA_LUT_INITIAL_SIZE: u64 = 1024;
const CHUNK_DATA_LUT_ELEMENT_SIZE: u64 = std::mem::size_of::<PackedGpuResourceId>() as u64;
const CHUNK_DATA_INITIAL_SIZE: u64 = 1024;
const CHUNK_DATA_ELEMENT_SIZE: u64 = std::mem::size_of::<VoxelOctreeNode>() as u64;
const VOXEL_DATA_INITIAL_SIZE: u64 = 1024;
const VOXEL_DATA_ELEMENT_SIZE: u64 = std::mem::size_of::<VoxelData>() as u64;

#[repr(C)]
pub struct RayMarchPushConstants {
    backbuffer_image: PackedGpuResourceId,
    camera_buffer: PackedGpuResourceId,
    chunk_tree: PackedGpuResourceId,
    chunk_data_lut: PackedGpuResourceId,
    voxel_data: PackedGpuResourceId,
}

impl RayMarchPushConstants {
    pub fn new(
        backbuffer_image: ImageId,
        camera_buffer: BufferId,
        voxel_pipeline: &VoxelPipeline,
    ) -> Self {
        Self {
            backbuffer_image: backbuffer_image.pack(),
            camera_buffer: camera_buffer.pack(),
            chunk_tree: voxel_pipeline.chunk_tree_buffer.pack(),
            chunk_data_lut: voxel_pipeline.chunk_data_lut_buffer.pack(),
            voxel_data: voxel_pipeline.voxel_data_buffer.pack(),
        }
    }
}

#[derive(Resource)]
pub struct VoxelPipeline {
    ray_march_pipeline: PipelineId,

    chunk_tree_buffer: BufferId,
    chunk_tree_buffer_capacity: u64,
    chunk_data_lut_buffer: BufferId,
    chunk_data_buffers: Vec<BufferId>,
    chunk_data_lut_buffer_capacity: u64,
    voxel_data_buffer: BufferId,
}

impl VoxelPipeline {
    pub fn new(
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        pipeline_manager: &mut PipelineManager,
        device: &mut Device,
    ) -> Self {
        let chunk_tree_buffer = device.create_buffer(BufferInfo {
            size: CHUNK_TREE_INITIAL_SIZE * CHUNK_TREE_ELEMENT_SIZE,
            usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
            name: "chunk_tree_buffer".to_owned(),
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let chunk_data_lut_buffer = device.create_buffer(BufferInfo {
            size: CHUNK_DATA_LUT_INITIAL_SIZE * CHUNK_DATA_LUT_ELEMENT_SIZE,
            usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
            name: "chunk_data_lut_buffer".to_owned(),
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let voxel_data_buffer = device.create_buffer(BufferInfo {
            size: VOXEL_DATA_INITIAL_SIZE * VOXEL_DATA_ELEMENT_SIZE,
            usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
            name: "voxel_data_buffer".to_owned(),
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });

        Self {
            ray_march_pipeline: pipeline_manager.create_compute_pipeline::<RayMarchPushConstants>(
                assets,
                watched_shaders,
                RAY_MARCH_PATH.to_owned(),
            ),
            chunk_tree_buffer,
            chunk_tree_buffer_capacity: CHUNK_TREE_INITIAL_SIZE,
            chunk_data_lut_buffer,
            chunk_data_buffers: Vec::new(),
            chunk_data_lut_buffer_capacity: CHUNK_DATA_INITIAL_SIZE,
            voxel_data_buffer,
        }
    }

    pub fn record_copy_commands(
        &mut self,
        vox_world: &VoxelWorld,
        device: &mut Device,
        command_recorder: &mut CommandRecorder,
    ) {
        // Do tree updates from world.
        if vox_world.did_tree_update() {
            let world_chunk_tree_size = vox_world.chunk_tree().nodes().len() as u64;
            if world_chunk_tree_size > self.chunk_tree_buffer_capacity {
                todo!("resize chunk tree gpu buffer automatically");
            }

            let staging_buffer_size = world_chunk_tree_size * CHUNK_TREE_ELEMENT_SIZE;
            let staging_buffer = device.create_buffer(BufferInfo {
                name: "chunk_tree_staging_buffer".to_owned(),
                size: staging_buffer_size,
                memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });

            let ptr = device.map_buffer_typed::<u32>(staging_buffer);
            unsafe {
                ptr.write(vox_world.chunk_tree().side_length());
            };
            let node_ptr = unsafe { ptr.offset(1) } as *mut ChunkOctreeNode;
            unsafe {
                node_ptr.copy_from(
                    vox_world.chunk_tree().nodes().as_ptr(),
                    world_chunk_tree_size as usize,
                )
            };

            command_recorder.copy_buffer_to_buffer(
                device,
                staging_buffer,
                0,
                self.chunk_tree_buffer,
                0,
                staging_buffer_size,
            );

            command_recorder.pipeline_barrier_buffer_transition(
                device,
                BufferTransition {
                    buffer: self.chunk_tree_buffer,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                },
            );

            command_recorder.destroy_buffer_deferred(staging_buffer);
        }

        let mut should_update_chunk_data_lut = false;
        for updated_chunk_id in vox_world.chunks_updated() {
            let chunk_buffer = match self.chunk_data_buffers.get(*updated_chunk_id as usize) {
                Some(buffer) => *buffer,
                None => {
                    let buffer = device.create_buffer(BufferInfo {
                        size: CHUNK_DATA_INITIAL_SIZE * CHUNK_DATA_ELEMENT_SIZE,
                        usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
                        name: format!("chunk_data_buffer_{}", updated_chunk_id).to_owned(),
                        memory_flags: MemoryFlags::DEVICE_LOCAL,
                    });
                    if *updated_chunk_id as usize >= self.chunk_data_buffers.len() {
                        self.chunk_data_buffers.push(buffer);
                    } else {
                        self.chunk_data_buffers[*updated_chunk_id as usize] = buffer;
                    }
                    should_update_chunk_data_lut = true;

                    buffer
                }
            };

            let chunk = &vox_world.chunk_data()[*updated_chunk_id as usize];

            let chunk_node_size = chunk.nodes().len() as u64;
            if chunk_node_size > self.chunk_data_lut_buffer_capacity {
                todo!("resize chunk data gpu buffer automatically");
            }

            let staging_buffer_size = chunk_node_size * CHUNK_DATA_ELEMENT_SIZE;
            let staging_buffer = device.create_buffer(BufferInfo {
                name: "chunk_data_staging_buffer".to_owned(),
                size: staging_buffer_size,
                memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });

            let ptr = device.map_buffer_typed::<VoxelOctreeNode>(staging_buffer);
            unsafe { ptr.copy_from(chunk.nodes().as_ptr(), chunk_node_size as usize) };

            command_recorder.copy_buffer_to_buffer(
                device,
                staging_buffer,
                0,
                chunk_buffer,
                0,
                staging_buffer_size,
            );

            command_recorder.pipeline_barrier_buffer_transition(
                device,
                BufferTransition {
                    buffer: chunk_buffer,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                },
            );

            command_recorder.destroy_buffer_deferred(staging_buffer);
        }

        if should_update_chunk_data_lut {
            let chunk_data_lut = self
                .chunk_data_buffers
                .iter()
                .map(|buffer_id| buffer_id.pack())
                .collect::<Vec<_>>();

            let chunk_data_lut_size = chunk_data_lut.len() as u64;
            if chunk_data_lut_size > self.chunk_data_lut_buffer_capacity {
                todo!("resize chunk data lut gpu buffer automatically");
            }

            let staging_buffer_size = chunk_data_lut_size * CHUNK_DATA_LUT_ELEMENT_SIZE;
            let staging_buffer = device.create_buffer(BufferInfo {
                name: "chunk_data_lut_staging_buffer".to_owned(),
                size: staging_buffer_size,
                memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });

            let ptr = device.map_buffer_typed::<PackedGpuResourceId>(staging_buffer);
            unsafe {
                ptr.copy_from(
                    chunk_data_lut.as_slice().as_ptr(),
                    chunk_data_lut_size as usize,
                )
            };

            command_recorder.copy_buffer_to_buffer(
                device,
                staging_buffer,
                0,
                self.chunk_data_lut_buffer,
                0,
                staging_buffer_size,
            );

            command_recorder.pipeline_barrier_buffer_transition(
                device,
                BufferTransition {
                    buffer: self.chunk_data_lut_buffer,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                },
            );
        }

        if vox_world.did_voxel_data_update() {
            let voxel_data_size = vox_world.voxel_data().len() as u64;
            if voxel_data_size > VOXEL_DATA_INITIAL_SIZE {
                todo!("resize voxel data gpu buffer automatically");
            }

            let staging_buffer_size = voxel_data_size * VOXEL_DATA_ELEMENT_SIZE;
            let staging_buffer = device.create_buffer(BufferInfo {
                name: "voxel_data_staging_buffer".to_owned(),
                size: staging_buffer_size,
                memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });

            let ptr = device.map_buffer_typed::<VoxelData>(staging_buffer);
            unsafe { ptr.copy_from(vox_world.voxel_data().as_ptr(), voxel_data_size as usize) };

            command_recorder.copy_buffer_to_buffer(
                device,
                staging_buffer,
                0,
                self.voxel_data_buffer,
                0,
                staging_buffer_size,
            );

            command_recorder.pipeline_barrier_buffer_transition(
                device,
                BufferTransition {
                    buffer: self.voxel_data_buffer,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                },
            );

            command_recorder.destroy_buffer_deferred(staging_buffer);
        }
    }

    pub fn ray_march_pipeline(&self) -> PipelineId {
        self.ray_march_pipeline
    }
}
