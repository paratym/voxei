use paya::{
    common::{
        AccessFlags, Extent2D, Extent3D, Format, ImageLayout, ImageTransition, ImageUsageFlags,
    },
    device::{ImageInfo, PresentInfo, SubmitInfo},
    gpu_resources::ImageId,
};
use voxei_macros::Resource;

use crate::{
    engine::{
        assets::watched_shaders::WatchedShaders,
        common::{camera::Camera, time::Time},
        ecs::ecs_world::ECSWorld,
        resource::{Res, ResMut},
        voxel::vox_world::VoxelWorld,
    },
    game::player::player::PlayerTag,
};

use super::{
    device::DeviceResource,
    pass::voxel::{RayMarchPushConstants, VoxelPipeline},
    pipeline_manager::PipelineManager,
    swapchain::SwapchainResource,
};

#[derive(Resource)]
pub struct RenderManager {
    backbuffer: Option<ImageId>,
}

impl RenderManager {
    pub fn new() -> Self {
        Self { backbuffer: None }
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
                usage: ImageUsageFlags::TRANSFER_SRC
                    | ImageUsageFlags::STORAGE
                    | ImageUsageFlags::COLOR_ATTACHMENT,
                format: Format::R8G8B8A8Unorm,
                ..Default::default()
            });

            render_manager.backbuffer = Some(image);
        }
    }

    pub fn render(
        render_manager: ResMut<RenderManager>,
        mut voxel_pipeline: ResMut<VoxelPipeline>,
        vox_world: Res<VoxelWorld>,
        pipeline_manager: Res<PipelineManager>,
        mut device: ResMut<DeviceResource>,
        mut swapchain: ResMut<SwapchainResource>,
        ecs_world: Res<ECSWorld>,
        time: Res<Time>,
    ) {
        let Some(image_index) = swapchain.acquire_next_image() else {
            return;
        };

        let Some(backbuffer_index) = render_manager.backbuffer else {
            return;
        };
        let backbuffer_info = device.get_image(backbuffer_index).info.clone();

        let Some(voxel_ray_march_pipeline) =
            pipeline_manager.get_compute_pipeline(voxel_pipeline.ray_march_pipeline())
        else {
            return;
        };

        let Some(primary_camera_buffer_id) = ecs_world
            .query::<&Camera>()
            .with::<&PlayerTag>()
            .iter()
            .next()
            .map(|(_, camera)| camera.buffer())
        else {
            return;
        };

        let mut command_recorder = device.create_command_recorder();

        voxel_pipeline.record_copy_commands(&vox_world, &mut device, &mut command_recorder);
        for (_, camera) in ecs_world.query::<&Camera>().iter() {
            camera.record_copy_commands(&mut device, &mut command_recorder);
        }

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
            command_recorder.bind_compute_pipeline(&device, voxel_ray_march_pipeline);
            command_recorder.upload_push_constants(
                &device,
                voxel_ray_march_pipeline,
                &RayMarchPushConstants::new(
                    backbuffer_index,
                    primary_camera_buffer_id,
                    &voxel_pipeline,
                ),
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
                dst_layout: ImageLayout::ColorAttachmentOptimal,
                dst_access: AccessFlags::SHADER_WRITE,
            },
        );

        command_recorder.pipeline_barrier_image_transition(
            &device,
            ImageTransition {
                image: backbuffer_index,
                src_layout: ImageLayout::ColorAttachmentOptimal,
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
