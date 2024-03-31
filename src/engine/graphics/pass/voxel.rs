use nalgebra::Vector3;
use paya::{
    allocator::MemoryFlags,
    command_recorder::{self, CommandRecorder},
    common::{
        AccessFlags, BufferTransition, BufferUsageFlags, Extent2D, ImageLayout, ImageTransition,
    },
    device::Device,
    gpu_resources::{BufferId, BufferInfo, ImageId, PackedGpuResourceId},
    swapchain::Swapchain,
};
use voxei_macros::Resource;

use crate::{
    constants,
    engine::{
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
        voxel::vox_world::{self, BrickIndex, VoxelWorld},
    },
};

const RAY_MARCH_PATH: &str = "shaders/ray_march.comp.glsl";

#[repr(C)]
struct WorldInfo {
    chunk_center: Vector3<i32>,
    chunk_occupancy_grid_buffer: PackedGpuResourceId,
    brick_indices_grid_buffer: PackedGpuResourceId,
    brick_data_buffer: PackedGpuResourceId,
    brick_request_list_buffer: PackedGpuResourceId,
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

const BRICK_REQUEST_BUFFER_SIZE_BYTES: usize = constants::MAX_BRICK_REQUEST * 4 + 4;
const BRICK_DATA_BUFFER_SIZE_BYTES: usize = (64 + 4) * 1024;

#[derive(Resource)]
pub struct VoxelPipeline {
    ray_march_pipeline: PipelineId,

    voxel_world_info_buffer: BufferId,
    chunk_occupancy_grid_buffer: BufferId,
    brick_indices_grid_buffer: BufferId,
    brick_data_buffer: BufferId,

    brick_request_staging_buffers: Vec<BufferId>,
    brick_request_list_buffer: BufferId,
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
        let brick_indices_grid_buffer = Self::create_brick_indices_grid_buffer(device, vox_world);
        let brick_data_buffer = create_device_buffer(
            device,
            "brick_data_buffer",
            BRICK_DATA_BUFFER_SIZE_BYTES as u64,
        );

        let brick_request_staging_buffers = (0..constants::MAX_FRAMES_IN_FLIGHT)
            .map(|i| {
                device.create_buffer(BufferInfo {
                    name: format!("brick_request_staging_buffer_{}", i),
                    size: BRICK_REQUEST_BUFFER_SIZE_BYTES as u64,
                    memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
                    usage: BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST,
                })
            })
            .collect();
        let brick_request_list_buffer = device.create_buffer(BufferInfo {
            name: "brick_request_list_buffer".to_owned(),
            size: BRICK_REQUEST_BUFFER_SIZE_BYTES as u64,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
            usage: BufferUsageFlags::STORAGE
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
        });

        Self {
            ray_march_pipeline: pipeline_manager.create_compute_pipeline::<RayMarchPushConstants>(
                assets,
                watched_shaders,
                RAY_MARCH_PATH.to_owned(),
            ),

            voxel_world_info_buffer,
            chunk_occupancy_grid_buffer,
            brick_indices_grid_buffer,
            brick_data_buffer,

            brick_request_staging_buffers,
            brick_request_list_buffer,
        }
    }

    fn create_chunk_occupancy_grid_buffer(device: &mut Device, vox_world: &VoxelWorld) -> BufferId {
        create_device_buffer(
            device,
            "chunk_occupancy_grid_buffer",
            vox_world.chunk_occupancy_grid().len() as u64,
        )
    }

    fn create_brick_indices_grid_buffer(device: &mut Device, vox_world: &VoxelWorld) -> BufferId {
        create_device_buffer(
            device,
            "brick_indices_grid_buffer",
            (vox_world.brick_indices_grid().len() * std::mem::size_of::<BrickIndex>()) as u64,
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
        // Upload entire world info buffer
        stage_buffer_copy(
            device,
            command_recorder,
            self.voxel_world_info_buffer,
            AccessFlags::SHADER_READ,
            |ptr: *mut WorldInfo| unsafe {
                ptr.write(WorldInfo {
                    chunk_center: vox_world.chunk_center(),
                    chunk_occupancy_grid_buffer: self.chunk_occupancy_grid_buffer.pack(),
                    brick_indices_grid_buffer: self.brick_indices_grid_buffer.pack(),
                    brick_data_buffer: self.brick_data_buffer.pack(),
                    brick_request_list_buffer: self.brick_request_list_buffer.pack(),
                    chunk_render_distance: vox_world.chunk_render_distance(),
                })
            },
        );

        // Upload entire chunk occupancy buffer.
        stage_buffer_copy(
            device,
            command_recorder,
            self.chunk_occupancy_grid_buffer,
            AccessFlags::SHADER_READ,
            |ptr: *mut u8| unsafe {
                ptr.copy_from_nonoverlapping(
                    vox_world.chunk_occupancy_grid().as_ptr(),
                    vox_world.chunk_occupancy_grid().len(),
                )
            },
        );

        // Upload entire brick indices buffer.
        stage_buffer_copy(
            device,
            command_recorder,
            self.brick_indices_grid_buffer,
            AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
            |ptr: *mut BrickIndex| unsafe {
                ptr.copy_from_nonoverlapping(
                    vox_world.brick_indices_grid().as_ptr(),
                    vox_world.brick_indices_grid().len(),
                )
            },
        );

        // Reset request list ptr.
        stage_buffer_copy(
            device,
            command_recorder,
            self.brick_request_list_buffer,
            AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
            |ptr: *mut u32| unsafe { ptr.write(0) },
        );
    }

    pub fn record_ray_march_commands(
        &self,
        device: &mut Device,
        command_recorder: &mut CommandRecorder,
        pipeline_manager: &PipelineManager,
        backbuffer_image: ImageId,
        backbuffer_extent: Extent2D,
        backbuffer_src_layout: ImageLayout,
        backbuffer_src_access: AccessFlags,
        camera_buffer: BufferId,
        cpu_frame_index: u64,
    ) {
        command_recorder.pipeline_barrier_image_transition(
            device,
            ImageTransition {
                image: backbuffer_image,
                src_layout: backbuffer_src_layout,
                src_access: backbuffer_src_access,
                dst_layout: ImageLayout::General,
                dst_access: AccessFlags::SHADER_WRITE,
            },
        );

        let pipeline = pipeline_manager
            .get_compute_pipeline(self.ray_march_pipeline)
            .unwrap();

        command_recorder.bind_compute_pipeline(&device, pipeline);
        command_recorder.upload_push_constants(
            &device,
            pipeline,
            &RayMarchPushConstants::new(backbuffer_image, camera_buffer, self),
        );
        command_recorder.dispatch(
            &device,
            (backbuffer_extent.width as f32 / 32.0).ceil() as u32,
            (backbuffer_extent.height as f32 / 32.0).ceil() as u32,
            1,
        );

        command_recorder.pipeline_barrier_buffer_transition(
            device,
            BufferTransition {
                buffer: self.brick_request_list_buffer,
                src_access: AccessFlags::SHADER_WRITE,
                dst_access: AccessFlags::TRANSFER_READ,
            },
        );

        let brick_request_staging_buffer = self.brick_request_staging_buffers
            [(cpu_frame_index + 1) as usize % constants::MAX_FRAMES_IN_FLIGHT];
        command_recorder.copy_buffer_to_buffer(
            device,
            self.brick_request_list_buffer,
            0,
            brick_request_staging_buffer,
            0,
            BRICK_REQUEST_BUFFER_SIZE_BYTES as u64,
        );
    }

    pub fn ray_march_pipeline(&self) -> PipelineId {
        self.ray_march_pipeline
    }

    pub fn brick_request_list_stage_buffer(&self, cpu_frame_index: u64) -> BufferId {
        self.brick_request_staging_buffers
            [cpu_frame_index as usize % constants::MAX_FRAMES_IN_FLIGHT]
    }
}
