use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

use nalgebra::Vector3;
use paya::{
    allocator::{MemoryFlags, MemoryLocation},
    command_recorder::{self, CommandRecorder, CopyRegion},
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
        voxel::{
            dynamic_world::{
                BrickChange, BrickData, BrickIndex, BrickPalette, DynVoxelWorld,
                PackedVoxelMaterial, SpatialStatus,
            },
            util::Morton,
            vox_constants::BRICK_VOLUME,
            vox_world::{DynBrickPos, VoxelWorld},
        },
    },
    settings::Settings,
};

const RAY_MARCH_PATH: &str = "shaders/ray_march.comp.glsl";
const NORMAL_CALC_PATH: &str = "shaders/normal_calc.comp.glsl";

#[repr(C)]
struct WorldInfo {
    chunk_center: Vector3<i32>,
    _padding0: u32,
    dyn_chunk_translation: Vector3<u32>,

    super_chunk_bit_mask_buffer: PackedGpuResourceId,
    chunk_occupancy_mask_buffer: PackedGpuResourceId,
    brick_indices_grid_buffer: PackedGpuResourceId,
    brick_data_buffer: PackedGpuResourceId,
    brick_palette_list_buffer: PackedGpuResourceId,
    brick_palette_indices_buffer: PackedGpuResourceId,
    brick_request_list_buffer: PackedGpuResourceId,

    dyn_chunk_side_length: u32,
    dyn_chunk_half_length: u32,
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

pub struct NormalCalcPushConstants {
    vox_world_buffer: PackedGpuResourceId,
    to_process_bricks_buffer: PackedGpuResourceId,
}

pub type BrickRequest = Morton;

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
    normal_calc_pipeline: PipelineId,

    voxel_world_info_buffer: BufferId,
    super_chunk_occupancy_grid_buffer: BufferId,
    chunk_occupancy_grid_buffer: BufferId,
    brick_indices_grid_buffer: BufferId,
    brick_data_buffer: BufferId,
    brick_palette_data_buffer: BufferId,
    brick_palette_indices_buffer: BufferId,

    brick_normal_process_list_buffer: BufferId,
    current_frame_brick_process_count: u32,

    brick_request_staging_buffers: Vec<BufferId>,
    brick_request_list_buffer: BufferId,

    queued_brick_updates: VecDeque<BrickChange>,
    queued_brick_normal_updates: VecDeque<BrickChange>,
}

impl VoxelPipeline {
    pub fn new(
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        pipeline_manager: &mut PipelineManager,
        device: &mut Device,
        settings: &Settings,
        vox_world: &VoxelWorld,
    ) -> Self {
        let voxel_world_info_buffer =
            create_device_buffer_typed::<WorldInfo>(device, "voxel_world_info_buffer");
        let super_chunk_occupancy_grid_buffer =
            Self::create_super_chunk_occupancy_grid_buffer(device, vox_world.dyn_world());
        let chunk_occupancy_grid_buffer =
            Self::create_chunk_occupancy_grid_buffer(device, vox_world.dyn_world());
        let brick_indices_grid_buffer =
            Self::create_brick_indices_grid_buffer(device, vox_world.dyn_world());

        let brick_data_buffer = Self::create_brick_data_buffer(device, settings);
        let brick_palette_data_buffer = Self::create_palette_data_buffer(device, settings);
        let brick_palette_indices_buffer = Self::create_palette_indices_buffer(device, settings);

        let brick_request_staging_buffers = (0..constants::MAX_FRAMES_IN_FLIGHT)
            .map(|i| Self::create_brick_request_list_staging_buffer(device, settings, i as u32))
            .collect();
        let brick_request_list_buffer = Self::create_brick_request_list_buffer(device, settings);
        let brick_normal_process_list_buffer =
            Self::create_brick_normal_process_list_buffer(device, settings);

        Self {
            ray_march_pipeline: pipeline_manager.create_compute_pipeline::<RayMarchPushConstants>(
                assets,
                watched_shaders,
                RAY_MARCH_PATH.to_owned(),
            ),
            normal_calc_pipeline: pipeline_manager
                .create_compute_pipeline::<NormalCalcPushConstants>(
                    assets,
                    watched_shaders,
                    NORMAL_CALC_PATH.to_owned(),
                ),

            voxel_world_info_buffer,
            super_chunk_occupancy_grid_buffer,
            chunk_occupancy_grid_buffer,
            brick_indices_grid_buffer,
            brick_data_buffer,
            brick_palette_data_buffer,
            brick_palette_indices_buffer,
            brick_normal_process_list_buffer,
            current_frame_brick_process_count: 0,

            brick_request_staging_buffers,
            brick_request_list_buffer,

            queued_brick_updates: VecDeque::new(),
            queued_brick_normal_updates: VecDeque::new(),
        }
    }

    pub fn update_world_changes(
        mut vox_world: ResMut<VoxelWorld>,
        mut vox_pipeline: ResMut<VoxelPipeline>,
    ) {
        vox_pipeline
            .queued_brick_updates
            .extend(vox_world.dyn_world_mut().collect_brick_changes());
        vox_pipeline
            .queued_brick_normal_updates
            .extend(vox_world.dyn_world_mut().collect_brick_normal_updates());
    }

    pub fn record_copy_commands(
        &mut self,
        vox_world: &mut VoxelWorld,
        device: &mut Device,
        command_recorder: &mut CommandRecorder,
        settings: &Settings,
    ) {
        let cpu_frame_index = device.cpu_frame_index();

        // Upload entire world info buffer
        stage_buffer_copy(
            device,
            command_recorder,
            self.voxel_world_info_buffer,
            AccessFlags::SHADER_READ,
            |ptr: *mut WorldInfo| unsafe {
                ptr.write(WorldInfo {
                    chunk_center: vox_world.chunk_center().vector,
                    super_chunk_bit_mask_buffer: self.super_chunk_occupancy_grid_buffer.pack(),
                    chunk_occupancy_mask_buffer: self.chunk_occupancy_grid_buffer.pack(),
                    brick_indices_grid_buffer: self.brick_indices_grid_buffer.pack(),
                    brick_data_buffer: self.brick_data_buffer.pack(),
                    brick_palette_list_buffer: self.brick_palette_data_buffer.pack(),
                    brick_palette_indices_buffer: self.brick_palette_indices_buffer.pack(),
                    brick_request_list_buffer: self.brick_request_list_buffer.pack(),

                    dyn_chunk_side_length: vox_world
                        .dyn_world()
                        .chunk_render_distance()
                        .pow2_side_length(),
                    dyn_chunk_half_length: vox_world
                        .dyn_world()
                        .chunk_render_distance()
                        .pow2_half_side_length(),
                    dyn_chunk_translation: vox_world.dyn_world().chunk_translation(),

                    _padding0: 0,
                })
            },
        );

        // Upload entire chunk occupancy buffer.
        stage_buffer_copy(
            device,
            command_recorder,
            self.super_chunk_occupancy_grid_buffer,
            AccessFlags::SHADER_READ,
            |ptr: *mut u8| unsafe {
                let grid_slice = vox_world.dyn_world().super_chunk_bit_grid().as_slice();
                ptr.copy_from_nonoverlapping(grid_slice.as_ptr(), grid_slice.len());
            },
        );
        stage_buffer_copy(
            device,
            command_recorder,
            self.chunk_occupancy_grid_buffer,
            AccessFlags::SHADER_READ,
            |ptr: *mut u8| unsafe {
                let grid_slice = vox_world.dyn_world().chunk_bit_grid().as_slice();
                ptr.copy_from_nonoverlapping(grid_slice.as_ptr(), grid_slice.len());
            },
        );

        let brick_change_upload_size =
            self.queued_brick_updates
                .len()
                .min(settings.brick_load_max_size as usize) as u64;
        if brick_change_upload_size > 0 {
            let brick_indices_staging_buffer = device.create_buffer(BufferInfo {
                name: format!("brick_indices_staging_buffer_{}", cpu_frame_index).to_owned(),
                size: std::mem::size_of::<BrickIndex>() as u64 * brick_change_upload_size,
                memory_location: MemoryLocation::CpuToGpu,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });
            let brick_data_staging_buffer = device.create_buffer(BufferInfo {
                name: format!("brick_data_staging_buffer_{}", cpu_frame_index).to_owned(),
                size: std::mem::size_of::<BrickData>() as u64 * brick_change_upload_size,
                memory_location: MemoryLocation::CpuToGpu,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });
            let brick_palette_staging_buffer = device.create_buffer(BufferInfo {
                name: format!("brick_material_data_staging_buffer_{}", cpu_frame_index).to_owned(),
                size: std::mem::size_of::<PackedVoxelMaterial>() as u64
                    * 256
                    * brick_change_upload_size,
                memory_location: MemoryLocation::CpuToGpu,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });
            let brick_palette_indices_staging_buffer = device.create_buffer(BufferInfo {
                name: format!("brick_palette_indices_staging_buffer_{}", cpu_frame_index)
                    .to_owned(),
                size: std::mem::size_of::<u8>() as u64
                    * BRICK_VOLUME as u64
                    * brick_change_upload_size,
                memory_location: MemoryLocation::CpuToGpu,
                usage: BufferUsageFlags::TRANSFER_SRC,
            });

            command_recorder.destroy_buffer_deferred(brick_indices_staging_buffer);
            command_recorder.destroy_buffer_deferred(brick_data_staging_buffer);
            command_recorder.destroy_buffer_deferred(brick_palette_staging_buffer);
            command_recorder.destroy_buffer_deferred(brick_palette_indices_staging_buffer);

            let brick_indices_staging_ptr =
                device.map_buffer_typed::<BrickIndex>(brick_indices_staging_buffer);
            let brick_data_staging_ptr =
                device.map_buffer_typed::<BrickData>(brick_data_staging_buffer);
            let mut brick_palette_staging_ptr =
                device.map_buffer_typed::<PackedVoxelMaterial>(brick_palette_staging_buffer);
            let mut brick_palette_staging_index = 0;
            let brick_palette_indices_staging_ptr =
                device.map_buffer_typed::<u8>(brick_palette_indices_staging_buffer);

            let mut brick_indices_copies = Vec::new();
            let mut brick_data_copies = Vec::new();
            let mut brick_palette_copies = Vec::new();
            let mut brick_palette_indices_copies = Vec::new();
            let time = Instant::now();
            for brick_update in self.queued_brick_updates.drain(
                (self.queued_brick_updates.len() as isize - brick_change_upload_size as isize)
                    .max(0) as usize..,
            ) {
                let brick_morton = *brick_update.brick_morton;
                let brick_index =
                    vox_world.dyn_world().brick_indices_grid().as_slice()[brick_morton as usize];

                // Set brick indices element
                let brick_indices_stage_index = brick_indices_copies.len();
                let indices_offset_ptr =
                    unsafe { brick_indices_staging_ptr.add(brick_indices_stage_index) };
                unsafe { indices_offset_ptr.write(brick_index) };
                brick_indices_copies.push(CopyRegion {
                    src_offset: brick_indices_stage_index as u64
                        * std::mem::size_of::<BrickIndex>() as u64,
                    dst_offset: brick_morton * std::mem::size_of::<BrickIndex>() as u64,
                    size: std::mem::size_of::<BrickIndex>() as u64,
                });
                if brick_morton
                    >= vox_world.dyn_world().brick_indices_grid().buffer_size() as u64 / 4
                {
                    // println!("Brick indices buffer on gpu is too small, can't add new bricks.");
                }

                if brick_index.status() == SpatialStatus::Loaded {
                    // Set brick data element
                    let brick_index = brick_index.index();
                    if brick_index >= settings.brick_data_max_size {
                        // println!("Brick data buffer on gpu is too small, can't add new bricks.");
                        continue;
                    }

                    let brick_data_stage_index = brick_data_copies.len();
                    let brick_data_offset_ptr =
                        unsafe { brick_data_staging_ptr.add(brick_data_stage_index) };
                    let brick_data = vox_world.dyn_world().brick_data().get(brick_index);
                    unsafe { brick_data_offset_ptr.copy_from(brick_data as *const _, 1) };
                    brick_data_copies.push(CopyRegion {
                        src_offset: brick_data_stage_index as u64
                            * std::mem::size_of::<BrickData>() as u64,
                        dst_offset: brick_index as u64 * std::mem::size_of::<BrickData>() as u64,
                        size: std::mem::size_of::<BrickData>() as u64,
                    });

                    let brick_palette_index = brick_data.palette_index();
                    if brick_palette_index >= settings.brick_palette_max_size * 256 {
                        // println!("Brick palette buffer on gpu is too small, can't add new bricks palettes.");
                        continue;
                    }

                    let brick_palette = vox_world
                        .dyn_world()
                        .brick_palette_list()
                        .get(brick_palette_index, brick_data.palette_size());
                    unsafe {
                        brick_palette_staging_ptr
                            .copy_from(brick_palette.as_ptr(), brick_palette.len())
                    };
                    brick_palette_copies.push(CopyRegion {
                        src_offset: brick_palette_staging_index
                            * std::mem::size_of::<PackedVoxelMaterial>() as u64,
                        dst_offset: brick_palette_index as u64
                            * std::mem::size_of::<PackedVoxelMaterial>() as u64,
                        size: std::mem::size_of::<PackedVoxelMaterial>() as u64
                            * brick_palette.len() as u64,
                    });

                    brick_palette_staging_index += brick_palette.len() as u64;
                    brick_palette_staging_ptr =
                        unsafe { brick_palette_staging_ptr.add(brick_palette.len() as usize) };

                    let brick_palette_indices =
                        vox_world.dyn_world().brick_data().get_indices(brick_index);
                    let brick_palette_indices_stage_index = brick_data_stage_index * BRICK_VOLUME;
                    let brick_palette_indices_offset_ptr = unsafe {
                        brick_palette_indices_staging_ptr
                            .add(brick_palette_indices_stage_index as usize)
                    };
                    unsafe {
                        brick_palette_indices_offset_ptr
                            .copy_from(brick_palette_indices.as_ptr(), BRICK_VOLUME)
                    };
                    brick_palette_indices_copies.push(CopyRegion {
                        src_offset: brick_palette_indices_stage_index as u64,
                        dst_offset: brick_index as u64 * BRICK_VOLUME as u64,
                        size: BRICK_VOLUME as u64,
                    });
                }
            }

            // println!("Time to copy bricks: {:?}", time.elapsed());
            command_recorder.copy_buffer_to_buffer_multiple(
                device,
                brick_indices_staging_buffer,
                self.brick_indices_grid_buffer,
                brick_indices_copies,
            );
            if brick_data_copies.len() > 0 {
                command_recorder.copy_buffer_to_buffer_multiple(
                    device,
                    brick_data_staging_buffer,
                    self.brick_data_buffer,
                    brick_data_copies,
                );
                command_recorder.copy_buffer_to_buffer_multiple(
                    device,
                    brick_palette_staging_buffer,
                    self.brick_palette_data_buffer,
                    brick_palette_copies,
                );
                command_recorder.copy_buffer_to_buffer_multiple(
                    device,
                    brick_palette_indices_staging_buffer,
                    self.brick_palette_indices_buffer,
                    brick_palette_indices_copies,
                );
            }
            // println!("Time to upload bricks: {:?}", time.elapsed());
        }

        let brick_normal_update_size =
            self.queued_brick_normal_updates
                .len()
                .min(settings.brick_load_max_size as usize) as u64;
        if brick_normal_update_size > 0 {
            let mut brick_normal_updates = Vec::new();
            for brick_normal_update in self.queued_brick_normal_updates.drain(
                (self.queued_brick_normal_updates.len() as isize
                    - brick_normal_update_size as isize)
                    .max(0) as usize..,
            ) {
                let brick_index = vox_world.dyn_world().brick_indices_grid().as_slice()
                    [*brick_normal_update.brick_morton as usize];
                if brick_index.status() == SpatialStatus::Loaded {
                    brick_normal_updates.push(*brick_normal_update.brick_morton as u32);
                }
            }

            if !brick_normal_updates.is_empty() {
                self.current_frame_brick_process_count = brick_normal_updates.len() as u32;
                stage_buffer_copy(
                    device,
                    command_recorder,
                    self.brick_normal_process_list_buffer,
                    AccessFlags::SHADER_READ,
                    |ptr: *mut u32| unsafe {
                        ptr.copy_from_nonoverlapping(
                            brick_normal_updates.as_ptr(),
                            brick_normal_updates.len(),
                        );
                    },
                )
            }
        }

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
        &mut self,
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

        if self.current_frame_brick_process_count > 0 {
            let pipeline = pipeline_manager
                .get_compute_pipeline(self.normal_calc_pipeline)
                .unwrap();

            command_recorder.bind_compute_pipeline(&device, pipeline);
            command_recorder.upload_push_constants(
                &device,
                pipeline,
                &NormalCalcPushConstants {
                    vox_world_buffer: self.voxel_world_info_buffer.pack(),
                    to_process_bricks_buffer: self.brick_normal_process_list_buffer.pack(),
                },
            );
            command_recorder.dispatch(&device, self.current_frame_brick_process_count, 1, 1);

            command_recorder.pipeline_barrier_buffer_transition(
                device,
                BufferTransition {
                    buffer: self.brick_palette_data_buffer,
                    src_access: AccessFlags::SHADER_WRITE | AccessFlags::SHADER_READ,
                    dst_access: AccessFlags::SHADER_READ,
                },
            );
            self.current_frame_brick_process_count = 0;
        }

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
            (backbuffer_extent.width as f32 / 8.0).ceil() as u32,
            (backbuffer_extent.height as f32 / 8.0).ceil() as u32,
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
        let brick_request_buffer_size = device.get_buffer(brick_request_staging_buffer).size;
        command_recorder.copy_buffer_to_buffer(
            device,
            self.brick_request_list_buffer,
            0,
            brick_request_staging_buffer,
            0,
            brick_request_buffer_size,
        );
    }

    pub fn ray_march_pipeline(&self) -> PipelineId {
        self.ray_march_pipeline
    }

    pub fn normal_calc_pipeline(&self) -> PipelineId {
        self.normal_calc_pipeline
    }

    pub fn compile_brick_requests(
        &self,
        device: &Device,
        compiled_brick_requests: &mut HashSet<Morton>,
        cpu_frame_index: u64,
        settings: &Settings,
    ) {
        let buffer = self.brick_request_staging_buffers
            [cpu_frame_index as usize % constants::MAX_FRAMES_IN_FLIGHT];
        let buffer_ptr = device.map_buffer_typed::<u32>(buffer);

        let size = unsafe { buffer_ptr.read() };
        assert!(size <= settings.brick_request_max_size);

        let ptr = unsafe { buffer_ptr.add(1) } as *mut Morton;
        for j in 0..size {
            compiled_brick_requests.insert(unsafe { ptr.add(j as usize).read() });
        }
    }

    fn create_super_chunk_occupancy_grid_buffer(
        device: &mut Device,
        vox_world: &DynVoxelWorld,
    ) -> BufferId {
        create_device_buffer(
            device,
            "super_chunk_occupancy_grid_buffer",
            vox_world.super_chunk_bit_grid().buffer_size() as u64,
        )
    }

    fn create_chunk_occupancy_grid_buffer(
        device: &mut Device,
        vox_world: &DynVoxelWorld,
    ) -> BufferId {
        create_device_buffer(
            device,
            "chunk_occupancy_grid_buffer",
            vox_world.chunk_bit_grid().buffer_size() as u64,
        )
    }

    fn create_brick_indices_grid_buffer(
        device: &mut Device,
        vox_world: &DynVoxelWorld,
    ) -> BufferId {
        create_device_buffer(
            device,
            "brick_indices_grid_buffer",
            vox_world.brick_indices_grid().buffer_size() as u64,
        )
    }

    fn create_brick_data_buffer(device: &mut Device, settings: &Settings) -> BufferId {
        create_device_buffer(
            device,
            "brick_data_buffer",
            std::mem::size_of::<BrickData>() as u64 * settings.brick_data_max_size as u64,
        )
    }

    fn create_palette_data_buffer(device: &mut Device, settings: &Settings) -> BufferId {
        create_device_buffer(
            device,
            "brick_palette_buffer",
            settings.brick_palette_max_size as u64
                * std::mem::size_of::<PackedVoxelMaterial>() as u64
                * 256,
        )
    }

    fn create_palette_indices_buffer(device: &mut Device, settings: &Settings) -> BufferId {
        create_device_buffer(
            device,
            "brick_palette_indices_buffer",
            settings.brick_data_max_size as u64 * BRICK_VOLUME as u64,
        )
    }

    fn create_brick_request_list_buffer(device: &mut Device, settings: &Settings) -> BufferId {
        device.create_buffer(BufferInfo {
            name: "brick_request_list_buffer".to_owned(),
            size: std::mem::size_of::<BrickRequest>() as u64
                * settings.brick_request_max_size as u64,
            memory_location: MemoryLocation::GpuOnly,
            usage: BufferUsageFlags::STORAGE
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
        })
    }

    fn create_brick_normal_process_list_buffer(
        device: &mut Device,
        settings: &Settings,
    ) -> BufferId {
        create_device_buffer(
            device,
            "brick_normal_process_list_buffer",
            std::mem::size_of::<u32>() as u64 * settings.brick_load_max_size as u64,
        )
    }

    fn create_brick_request_list_staging_buffer(
        device: &mut Device,
        settings: &Settings,
        index: u32,
    ) -> BufferId {
        device.create_buffer(BufferInfo {
            name: format!("brick_request_list_staging_buffer_{}", index).to_owned(),
            size: std::mem::size_of::<BrickRequest>() as u64
                * settings.brick_request_max_size as u64,
            memory_location: MemoryLocation::GpuToCpu,
            usage: BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST,
        })
    }
}
