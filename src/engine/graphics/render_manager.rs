use paya::{
    common::{
        AccessFlags, Extent2D, Extent3D, Format, ImageLayout, ImageTransition, ImageUsageFlags,
    },
    device::{Device, ImageInfo, PresentInfo, SubmitInfo},
    gpu_resources::ImageId,
};
use voxei_macros::Resource;

use crate::engine::{
    assets::{asset::Assets, watched_shaders::WatchedShaders},
    resource::{Res, ResMut},
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
        render_manager: ResMut<RenderManager>,
        mut device: ResMut<DeviceResource>,
        mut swapchain: ResMut<SwapchainResource>,
    ) {
        let Some(image_index) = swapchain.acquire_next_image() else {
            return;
        };

        let Some(backbuffer_index) = render_manager.backbuffer else {
            return;
        };

        let Some(voxel_ray_march_pipeline) = &render_manager.voxel_ray_march_pipeline.pipeline()
        else {
            return;
        };

        let mut command_recorder = device.create_command_recorder();

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
        command_recorder.bind_compute_pipeline(&device, voxel_ray_march_pipeline);
        command_recorder.upload_push_constants(
            &device,
            voxel_ray_march_pipeline,
            &RayMarchPushConstants {
                backbuffer_image: device.get_storage_image_resource_id(backbuffer_index),
            },
        );

        command_recorder.dispatch(&device, 8, 8, 1);

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
        })
    }
}
