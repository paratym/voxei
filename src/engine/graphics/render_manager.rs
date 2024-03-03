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
    constants,
    engine::{
        assets::{asset::Assets, watched_shaders::WatchedShaders},
        common::camera::{CameraBuffer, PrimaryCamera},
        resource::{Res, ResMut},
    },
};

use super::{
    device::DeviceResource,
    swapchain::SwapchainResource,
    voxel::{RayMarchPushConstants, VoxelRayMarchPipeline},
};

#[derive(Resource)]
pub struct RenderManager {
    backbuffer: Option<ImageId>,
    voxel_ray_march_pipeline: VoxelRayMarchPipeline,
}

impl RenderManager {
    pub fn new(assets: &mut Assets, watched_shaders: &mut WatchedShaders) -> Self {
        Self {
            backbuffer: None,
            voxel_ray_march_pipeline: VoxelRayMarchPipeline::new(assets, watched_shaders),
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
            .voxel_ray_march_pipeline
            .update(&device, &watched_shaders);
    }

    pub fn render(
        mut render_manager: ResMut<RenderManager>,
        mut device: ResMut<DeviceResource>,
        mut swapchain: ResMut<SwapchainResource>,
        primary_camera: Res<PrimaryCamera>,
    ) {
        let Some(image_index) = swapchain.acquire_next_image() else {
            return;
        };

        let Some(backbuffer_index) = render_manager.backbuffer else {
            return;
        };
        let backbuffer_info = device.get_image(backbuffer_index).info.clone();

        if render_manager.voxel_ray_march_pipeline.pipeline().is_none() {
            return;
        }

        let mut command_recorder = device.create_command_recorder();

        let staging_buffer = device.create_buffer(BufferInfo {
            size: std::mem::size_of::<CameraBuffer>() as u64,
            memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
            usage: BufferUsageFlags::TRANSFER_SRC,
        });

        // Update camera buffer
        {
            let ptr = device.map_buffer_typed::<CameraBuffer>(staging_buffer);
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
        }
        let camera_buffer_id = primary_camera
            .buffer(device.cpu_frame_index() % constants::MAX_FRAMES_IN_FLIGHT as u64);

        command_recorder.copy_buffer_to_buffer(
            &device,
            staging_buffer,
            camera_buffer_id,
            std::mem::size_of::<CameraBuffer>() as u64,
        );

        command_recorder.destroy_buffer_deferred(staging_buffer);

        command_recorder.pipeline_barrier_buffer_transition(
            &device,
            BufferTransition {
                buffer: camera_buffer_id,
                src_access: AccessFlags::TRANSFER_WRITE,
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
        let voxel_ray_march_pipeline = render_manager.voxel_ray_march_pipeline.pipeline().unwrap();
        command_recorder.bind_compute_pipeline(&device, voxel_ray_march_pipeline);
        command_recorder.upload_push_constants(
            &device,
            voxel_ray_march_pipeline,
            &RayMarchPushConstants {
                backbuffer_image: backbuffer_index.pack(),
                camera_buffer: camera_buffer_id.pack(),
            },
        );

        command_recorder.dispatch(
            &device,
            (backbuffer_info.extent.width as f32 / 16.0).ceil() as u32,
            (backbuffer_info.extent.height as f32 / 16.0).ceil() as u32,
            1,
        );

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
