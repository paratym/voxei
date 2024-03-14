use paya::{
    allocator::MemoryFlags,
    common::{
        AccessFlags, BufferTransition, BufferUsageFlags, Extent2D, Extent3D, Format, ImageLayout,
        ImageTransition, ImageUsageFlags,
    },
    device::{Device, ImageInfo, PresentInfo, SubmitInfo},
    gpu_resources::{BufferId, BufferInfo, ImageId},
};
use voxei_macros::Resource;

use crate::{
    constants::{self, VOXEL_LENGTH},
    engine::{
        assets::{asset::Assets, watched_shaders::WatchedShaders},
        common::{
            camera::{CameraBuffer, PrimaryCamera},
            time::Time,
        },
        resource::{Res, ResMut},
    },
};

use super::{
    common::{GTriangle, Vertex},
    device::DeviceResource,
    swapchain::SwapchainResource,
    voxel::{BuildSVOPushConstants, RayMarchPushConstants, VoxelPipeline, VoxelizePushConstants},
};

#[derive(Resource)]
pub struct RenderManager {
    backbuffer: Option<ImageId>,
    voxel_pipeline: VoxelPipeline,

    vertices_buffer: BufferId,
    indices_buffer: BufferId,
    voxel_buffer: BufferId,
    svo_buffer: BufferId,
    voxel_model_buffer: BufferId,
}

#[repr(C)]
struct VoxelModelBuffer {
    bounds: ((f32, f32, f32, f32), (f32, f32, f32, f32)),
    unit_length: f32,
    subdivisions: u32,
    node_length: u32,
}

#[repr(C)]
struct VoxelNode {
    data_index: u32,
    child_index: u32,
    child_offsets: u64,
}

#[repr(C)]
struct SVONode {
    _glsl_content: (u32, u32, u32, u32),
}

const SIDE_LENGTH: u32 = 32;
const VOXEL_ARRAY_SIZE: u32 = SIDE_LENGTH * SIDE_LENGTH * SIDE_LENGTH;
const SVO_ARRAY_SIZE: u64 = (VOXEL_ARRAY_SIZE * 2) as u64;
const VERTICES_BUFFER_SIZE: u64 = (std::mem::size_of::<f32>() * 9) as u64;
const INDICES_BUFFER_SIZE: u64 = (std::mem::size_of::<u32>() * 3) as u64;

impl RenderManager {
    pub fn new(
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        device: &mut Device,
    ) -> Self {
        let vertices_buffer = device.create_buffer(BufferInfo {
            name: "vertices_buffer".to_owned(),
            size: VERTICES_BUFFER_SIZE,
            usage: BufferUsageFlags::STORAGE
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::VERTEX,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let indices_buffer = device.create_buffer(BufferInfo {
            name: "indices_buffer".to_owned(),
            size: INDICES_BUFFER_SIZE,
            usage: BufferUsageFlags::STORAGE
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::INDEX,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let voxel_buffer = device.create_buffer(BufferInfo {
            name: "voxel_buffer".to_owned(),
            size: VOXEL_ARRAY_SIZE as u64 * 4,
            usage: BufferUsageFlags::STORAGE,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let svo_buffer = device.create_buffer(BufferInfo {
            name: "svo_buffer".to_owned(),
            size: SVO_ARRAY_SIZE * std::mem::size_of::<SVONode>() as u64 + 4,
            usage: BufferUsageFlags::STORAGE,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });
        let voxel_model_buffer = device.create_buffer(BufferInfo {
            name: "svo_model_buffer".to_owned(),
            size: (std::mem::size_of::<VoxelModelBuffer>() + std::mem::size_of::<VoxelNode>() * 3)
                as u64,
            usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
            memory_flags: MemoryFlags::DEVICE_LOCAL,
        });

        Self {
            backbuffer: None,
            voxel_pipeline: VoxelPipeline::new(assets, watched_shaders),
            vertices_buffer,
            indices_buffer,
            voxel_buffer,
            svo_buffer,
            voxel_model_buffer,
        }
    }

    pub fn update(
        mut render_manager: ResMut<RenderManager>,
        mut device: ResMut<DeviceResource>,
        swapchain: ResMut<SwapchainResource>,
        watched_shaders: Res<WatchedShaders>,
    ) {
        let backbuffer_size = render_manager
            .backbuffer
            .map(|image| device.get_image(image).info.extent);
        if render_manager.backbuffer.is_none()
            || Extent2D::from(backbuffer_size.unwrap()) != swapchain.info().extent
        {
            if let Some(backbuffer_image_id) = render_manager.backbuffer {
                device.destroy_image(backbuffer_image_id);
            }

            let image = device.create_image(ImageInfo {
                extent: Extent3D::from(swapchain.info().extent),
                usage: ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::STORAGE,
                format: Format::R8G8B8A8Unorm,
                ..Default::default()
            });

            render_manager.backbuffer = Some(image);
        }

        render_manager
            .voxel_pipeline
            .update(&device, &watched_shaders);
    }

    pub fn render(
        render_manager: ResMut<RenderManager>,
        mut device: ResMut<DeviceResource>,
        mut swapchain: ResMut<SwapchainResource>,
        primary_camera: Res<PrimaryCamera>,
        time: Res<Time>,
    ) {
        let Some(image_index) = swapchain.acquire_next_image() else {
            return;
        };

        let Some(backbuffer_index) = render_manager.backbuffer else {
            return;
        };
        let backbuffer_info = device.get_image(backbuffer_index).info.clone();

        if render_manager.voxel_pipeline.ray_march_pipeline().is_none()
            || render_manager.voxel_pipeline.voxelize_pipeline().is_none()
            || render_manager.voxel_pipeline.build_svo_pipeline().is_none()
        {
            return;
        }

        let mut command_recorder = device.create_command_recorder();

        let staging_buffer = device.create_buffer(BufferInfo {
            name: "camera_staging_buffer".to_owned(),
            size: std::mem::size_of::<CameraBuffer>() as u64,
            memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
            usage: BufferUsageFlags::TRANSFER_SRC,
        });

        let rot_per_sec = 0.25;
        let delta_angle = (std::f32::consts::PI * 2.0)
            * time.total_duration().as_millis() as f32
            * 0.001
            * rot_per_sec;
        let v0_angle = 0.0 + delta_angle;
        let v1_angle = std::f32::consts::FRAC_PI_3 * 2.0 + delta_angle;
        let v2_angle = std::f32::consts::FRAC_PI_3 * 4.0 + delta_angle;
        let radius = 10.0;
        let vertices = vec![
            // V0
            radius * v0_angle.cos(),
            0.5,
            radius * v0_angle.sin(),
            // V1
            radius * v1_angle.cos(),
            0.5,
            radius * v1_angle.sin(),
            // V2
            radius * v2_angle.cos(),
            0.5,
            radius * v2_angle.sin(),
        ];
        let indices = vec![0, 1, 2];

        let triangle_staging_buffer = device.create_buffer(BufferInfo {
            name: "triangle_staging_buffer".to_owned(),
            size: INDICES_BUFFER_SIZE + VERTICES_BUFFER_SIZE,
            memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
            usage: BufferUsageFlags::TRANSFER_SRC,
        });

        let voxel_staging_buffer = device.create_buffer(BufferInfo {
            name: "voxel_staging_buffer".to_owned(),
            size: (std::mem::size_of::<VoxelModelBuffer>() + std::mem::size_of::<VoxelNode>() * 3)
                as u64,
            memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
            usage: BufferUsageFlags::TRANSFER_SRC,
        });

        // Update buffers
        {
            let mapped_ptr = device.map_buffer_typed::<CameraBuffer>(staging_buffer);
            let ptr = mapped_ptr.clone();
            unsafe {
                ptr.write(CameraBuffer {
                    view_matrix: primary_camera
                        .camera()
                        .view()
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    resolution: primary_camera.resolution(),
                    aspect: primary_camera.aspect_ratio(),
                    fov: primary_camera.fov(),
                })
            };

            let mapped_ptr = device.map_buffer_typed::<u32>(triangle_staging_buffer);
            let ptr = mapped_ptr.clone();
            unsafe {
                let mut ptr = ptr as *mut u32;
                for index in indices.iter() {
                    ptr.write(*index);
                    ptr = ptr.offset(1);
                }
                let mut ptr = ptr as *mut f32;
                for vertex in vertices.iter() {
                    ptr.write(*vertex);
                    ptr = ptr.offset(1);
                }
            }

            let mapped_ptr = device.map_buffer_typed::<VoxelModelBuffer>(voxel_staging_buffer);
            let ptr = mapped_ptr.clone();
            unsafe {
                ptr.write(VoxelModelBuffer {
                    bounds: ((0.0, 0.0, 0.0, 0.0), (1.0, 1.0, 1.0, 0.0)),
                    unit_length: 0.5,
                    subdivisions: 1,
                    node_length: 3,
                });
                let ptr = ptr.offset(1) as *mut VoxelNode;
                // First node needs to be null
                ptr.write(VoxelNode {
                    data_index: 0,
                    child_index: 0,
                    child_offsets: 0xFFFFFFFFFFFFFFFF,
                });
                // Child node with "data"
                let ptr = ptr.offset(1);
                ptr.write(VoxelNode {
                    data_index: 1,
                    child_index: 0,
                    child_offsets: 0xFFFFFFFFFFFFFFFF,
                });
                // Root node
                let ptr = ptr.offset(1);
                ptr.write(VoxelNode {
                    data_index: 0,
                    child_index: 1,
                    child_offsets: 0xFFFFFFFFFFFFFF00,
                });
            }
        }
        let camera_buffer_id = primary_camera
            .buffer(device.cpu_frame_index() % constants::MAX_FRAMES_IN_FLIGHT as u64);

        command_recorder.copy_buffer_to_buffer(
            &device,
            staging_buffer,
            0,
            camera_buffer_id,
            0,
            std::mem::size_of::<CameraBuffer>() as u64,
        );
        command_recorder.copy_buffer_to_buffer(
            &device,
            triangle_staging_buffer,
            0,
            render_manager.indices_buffer,
            0,
            INDICES_BUFFER_SIZE,
        );
        command_recorder.copy_buffer_to_buffer(
            &device,
            triangle_staging_buffer,
            INDICES_BUFFER_SIZE,
            render_manager.vertices_buffer,
            0,
            VERTICES_BUFFER_SIZE,
        );
        command_recorder.copy_buffer_to_buffer(
            &device,
            voxel_staging_buffer,
            0,
            render_manager.voxel_model_buffer,
            0,
            (std::mem::size_of::<VoxelModelBuffer>() + std::mem::size_of::<VoxelNode>() * 3) as u64,
        );

        command_recorder.destroy_buffer_deferred(staging_buffer);
        command_recorder.destroy_buffer_deferred(triangle_staging_buffer);
        command_recorder.destroy_buffer_deferred(voxel_staging_buffer);

        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: camera_buffer_id,
                src_access: AccessFlags::TRANSFER_WRITE,
                dst_access: AccessFlags::SHADER_READ,
            },
        );
        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: render_manager.vertices_buffer,
                src_access: AccessFlags::TRANSFER_WRITE,
                dst_access: AccessFlags::SHADER_READ,
            },
        );
        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: render_manager.indices_buffer,
                src_access: AccessFlags::TRANSFER_WRITE,
                dst_access: AccessFlags::SHADER_READ,
            },
        );

        // Voxelization pass
        {
            let voxelize_pipeline = render_manager.voxel_pipeline.voxelize_pipeline().unwrap();
            command_recorder.bind_compute_pipeline(&device, voxelize_pipeline);
            command_recorder.upload_push_constants(
                &device,
                voxelize_pipeline,
                &VoxelizePushConstants {
                    vertices_buffer: render_manager.vertices_buffer.pack(),
                    indices_buffer: render_manager.indices_buffer.pack(),
                    voxel_buffer: render_manager.voxel_buffer.pack(),
                    side_length: SIDE_LENGTH,
                    triangle_count: indices.len() as u32 / 3,
                },
            );
            command_recorder.dispatch(
                &device,
                (VOXEL_ARRAY_SIZE as f32 / 16.0).ceil() as u32,
                1,
                1,
            );
        }

        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: render_manager.voxel_buffer,
                src_access: AccessFlags::SHADER_WRITE,
                dst_access: AccessFlags::SHADER_READ,
            },
        );

        // SVO building pass
        {
            let build_svo_pipeline = render_manager.voxel_pipeline.build_svo_pipeline().unwrap();
            command_recorder.bind_compute_pipeline(&device, build_svo_pipeline);

            let subdivisions = f32::log2(SIDE_LENGTH as f32) as u32;
            let mut head = 1;
            for i in (0..=subdivisions).rev() {
                let side_length = 1 << i;
                let size = side_length * side_length * side_length;
                command_recorder.upload_push_constants(
                    &device,
                    build_svo_pipeline,
                    &BuildSVOPushConstants {
                        voxel_buffer: render_manager.voxel_buffer.pack(),
                        svo_buffer: render_manager.svo_buffer.pack(),
                        head,
                        size,
                    },
                );
                command_recorder.dispatch(&device, (size as f32 / 16.0).ceil() as u32, 1, 1);

                head += size;
            }
        }

        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: render_manager.svo_buffer,
                src_access: AccessFlags::SHADER_WRITE,
                dst_access: AccessFlags::SHADER_READ,
            },
        );

        command_recorder.pipeline_barrier_image_transition(
            &device,
            ImageTransition {
                image: backbuffer_index,
                src_layout: ImageLayout::Undefined,
                src_access: AccessFlags::empty(),
                dst_layout: ImageLayout::General,
                dst_access: AccessFlags::SHADER_WRITE,
            },
        );

        // Ray march pass
        {
            let voxel_ray_march_pipeline =
                render_manager.voxel_pipeline.ray_march_pipeline().unwrap();
            command_recorder.bind_compute_pipeline(&device, voxel_ray_march_pipeline);
            command_recorder.upload_push_constants(
                &device,
                voxel_ray_march_pipeline,
                &RayMarchPushConstants {
                    backbuffer_image: backbuffer_index.pack(),
                    camera_buffer: camera_buffer_id.pack(),
                    voxel_model_buffer: render_manager.svo_buffer.pack(),
                    subdivisions: f32::log2(SIDE_LENGTH as f32) as u32,
                },
            );
            command_recorder.dispatch(
                &device,
                (backbuffer_info.extent.width as f32 / 16.0).ceil() as u32,
                (backbuffer_info.extent.height as f32 / 16.0).ceil() as u32,
                1,
            );
        }

        command_recorder.pipeline_barrier_image_transition(
            &device,
            ImageTransition {
                image: backbuffer_index,
                src_layout: ImageLayout::General,
                src_access: AccessFlags::SHADER_WRITE,
                dst_layout: ImageLayout::TransferSrcOptimal,
                dst_access: AccessFlags::TRANSFER_READ,
            },
        );
        command_recorder.pipeline_barrier_image_transition(
            &device,
            ImageTransition {
                image: image_index,
                src_layout: ImageLayout::Undefined,
                src_access: AccessFlags::empty(),
                dst_layout: ImageLayout::TransferDstOptimal,
                dst_access: AccessFlags::TRANSFER_WRITE,
            },
        );

        command_recorder.blit_image_to_image(&device, backbuffer_index, image_index);

        command_recorder.pipeline_barrier_image_transition(
            &device,
            ImageTransition {
                image: image_index,
                src_layout: ImageLayout::TransferDstOptimal,
                src_access: AccessFlags::TRANSFER_WRITE,
                dst_layout: ImageLayout::PresentSrc,
                dst_access: AccessFlags::empty(),
            },
        );

        let command_list = command_recorder.finish(&device);

        let signal_index = device.cpu_frame_index() as u64 + 1;
        device.submit(SubmitInfo {
            commands: vec![command_list],
            wait_semaphores: vec![swapchain.current_acquire_semaphore()],
            signal_semaphores: vec![swapchain.current_present_semaphore()],
            signal_timeline_semaphores: vec![(swapchain.gpu_timeline_semaphore(), signal_index)],
        });

        device.present(PresentInfo {
            swapchain: &swapchain,
            wait_semaphores: vec![swapchain.current_present_semaphore()],
        });

        device.collect_garbage(swapchain.gpu_timeline_semaphore());
    }

    pub fn backbuffer(&self) -> Option<ImageId> {
        self.backbuffer
    }
}
