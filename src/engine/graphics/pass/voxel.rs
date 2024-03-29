use nalgebra::Vector3;
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
    graphics::{
        device::{
            create_device_buffer, create_device_buffer_typed, stage_buffer_copy, DeviceResource,
        },
        pipeline_manager::{self, PipelineId, PipelineManager},
    },
    resource::{Res, ResMut},
    voxel::vox_world::{self, VoxelWorld},
};

const RAY_MARCH_PATH: &str = "shaders/ray_march.comp.glsl";

#[repr(C)]
struct WorldInfo {
    chunk_center: Vector3<i32>,
    chunk_occupancy_grid_buffer: PackedGpuResourceId,
    chunk_render_distance: u32,
}

#[repr(C)]
pub struct RayMarchPushConstants {
    backbuffer_image: PackedGpuResourceId,
    camera_buffer: PackedGpuResourceId,
    vox_world_buffer: PackedGpuResourceId,
    //     chunk_tree: PackedGpuResourceId,
    //     chunk_data_lut: PackedGpuResourceId,
    //     voxel_data: PackedGpuResourceId,
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
            vox_world_buffer: voxel_pipeline.voxel_world_info_buffer.pack(),
        }
    }
}

#[derive(Resource)]
pub struct VoxelPipeline {
    ray_march_pipeline: PipelineId,

    voxel_world_info_buffer: BufferId,
    chunk_occupancy_grid_buffer: BufferId,
}

impl VoxelPipeline {
    pub fn new(
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        pipeline_manager: &mut PipelineManager,
        device: &mut Device,
        vox_world: &VoxelWorld,
    ) -> Self {
        let voxel_world_info_buffer =
            create_device_buffer_typed::<WorldInfo>(device, "voxel_world_info_buffer");
        let chunk_occupancy_grid_buffer =
            Self::create_chunk_occupancy_grid_buffer(device, vox_world);

        Self {
            ray_march_pipeline: pipeline_manager.create_compute_pipeline::<RayMarchPushConstants>(
                assets,
                watched_shaders,
                RAY_MARCH_PATH.to_owned(),
            ),

            voxel_world_info_buffer,
            chunk_occupancy_grid_buffer,
        }
    }

    fn create_chunk_occupancy_grid_buffer(device: &mut Device, vox_world: &VoxelWorld) -> BufferId {
        create_device_buffer(
            device,
            "chunk_occupancy_grid_buffer",
            vox_world.chunk_occupancy_grid().len() as u64,
        )
    }

    pub fn update_resize_buffers(
        mut pipeline: ResMut<VoxelPipeline>,
        vox_world: Res<VoxelWorld>,
        mut device: ResMut<DeviceResource>,
    ) {
        if vox_world.render_distance_changed() {
            pipeline.chunk_occupancy_grid_buffer =
                Self::create_chunk_occupancy_grid_buffer(&mut device, &vox_world);
        }
    }

    pub fn record_copy_commands(
        &mut self,
        vox_world: &VoxelWorld,
        device: &mut Device,
        command_recorder: &mut CommandRecorder,
    ) {
        stage_buffer_copy(
            device,
            command_recorder,
            self.voxel_world_info_buffer,
            |ptr: *mut WorldInfo| unsafe {
                ptr.write(WorldInfo {
                    chunk_occupancy_grid_buffer: self.chunk_occupancy_grid_buffer.pack(),
                    chunk_render_distance: vox_world.chunk_render_distance(),
                    chunk_center: vox_world.chunk_center(),
                })
            },
        );
        stage_buffer_copy(
            device,
            command_recorder,
            self.chunk_occupancy_grid_buffer,
            |ptr: *mut u8| unsafe {
                ptr.copy_from_nonoverlapping(
                    vox_world.chunk_occupancy_grid().as_ptr(),
                    vox_world.chunk_occupancy_grid().len(),
                )
            },
        );
    }

    pub fn ray_march_pipeline(&self) -> PipelineId {
        self.ray_march_pipeline
    }
}
