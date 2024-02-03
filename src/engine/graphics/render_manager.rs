use ash::vk;
use voxei_macros::Resource;

use crate::{
    constants,
    engine::{
        graphics::vulkan::{
            executor::QueueExecutorSubmitInfo, swapchain::Swapchain, vulkan::Vulkan,
        },
        resource::{ResMut, Res}, window::window::Window,
    }, game::graphics::pipeline::util::refresh_render_resources,
};

use super::{
    queues::DefaultQueueExecutor,
    resource_manager::RenderResourceManager,
    vulkan::{objects::{
        command::{util::BlitImageInfo, CommandBuffer, CommandBufferHandle, CommandPool},
        image::ImageMemoryBarrier,
        sync::{Fence, Semaphore},
    }, swapchain::SwapchainCreateInfo}, SwapchainRefreshed,
};

#[derive(Resource)]
pub struct FrameIndex(usize);

impl FrameIndex {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next(&mut self) {
        self.0 = (self.0 + 1) % constants::FRAMES_IN_FLIGHT;
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

pub struct FrameResources {
    fence: Fence,
    render_finished_semaphore: Semaphore,
    image_available_semaphore: Semaphore,
    ready_to_present_semaphore: Semaphore,
    command_pool: CommandPool,
    main_command_buffer: CommandBufferHandle,
    blit_command_buffer: CommandBufferHandle,
}

#[derive(Debug, Clone)]
pub struct FrameSubmitInfo {
    pub submit_image: String,
    pub submit_image_last_layout: vk::ImageLayout,
    pub submit_image_last_access: vk::AccessFlags,
    pub last_stage: vk::PipelineStageFlags,
}

#[derive(Resource)]
pub struct RenderManager {
    frame_resources: [FrameResources; constants::FRAMES_IN_FLIGHT],
    submit_info: Option<FrameSubmitInfo>,
    swapchain_image_index: Option<u32>,
}

impl RenderManager {
    pub fn new(vulkan: &Vulkan) -> Self {
        let frame_resources = (0..constants::FRAMES_IN_FLIGHT)
            .map(|_| {
                let fence = Fence::new(vulkan, true);
                let image_available_semaphore = Semaphore::new(vulkan);
                let render_finished_semaphore = Semaphore::new(vulkan);
                let ready_to_present_semaphore = Semaphore::new(vulkan);

                let mut command_pool = CommandPool::new(vulkan);
                let [main_command_buffer, blit_command_buffer] = command_pool.allocate();

                FrameResources {
                    fence,
                    image_available_semaphore,
                    render_finished_semaphore,
                    ready_to_present_semaphore,
                    command_pool,
                    main_command_buffer,
                    blit_command_buffer,
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to create frames in flight."));

        Self {
            frame_resources,
            submit_info: None,
            swapchain_image_index: None,
        }
    }

    pub fn begin_frame(
        mut render_manager: ResMut<RenderManager>,
        frame_index: ResMut<FrameIndex>,
        swapchain: ResMut<Swapchain>,
        mut default_executor: ResMut<DefaultQueueExecutor>,
    ) {
        let current_frame = &mut render_manager.frame_resources[frame_index.index()];

        current_frame.fence.wait();

        let swapchain_image_index = 
            swapchain
                .get_next_image_index(&current_frame.image_available_semaphore)
                .expect("Failed to get next iamge index, probably need to implement fixing out of date swapchains.# vim");

        current_frame.fence.reset();
        current_frame.command_pool.reset();
        default_executor.release_frame_resources(frame_index.index());

        let main_command_buffer = current_frame
            .command_pool
            .get_mut(current_frame.main_command_buffer)
            .unwrap();

        main_command_buffer.begin();

        render_manager.swapchain_image_index = Some(swapchain_image_index);
    }

    pub fn main_command_buffer(&mut self, frame_index: usize) -> &mut CommandBuffer {
        let current_frame = &mut self.frame_resources[frame_index];
        current_frame
            .command_pool
            .get_mut(current_frame.main_command_buffer)
            .unwrap()
    }

    pub fn submit_frame(
        mut render_manager: ResMut<RenderManager>,
        resource_manager: ResMut<RenderResourceManager>,
        mut frame_index: ResMut<FrameIndex>,
        mut default_executor: ResMut<DefaultQueueExecutor>,
        vulkan: Res<Vulkan>,
        window: Res<Window>,
        mut swapchain: ResMut<Swapchain>,
        mut swapchain_refreshed: ResMut<SwapchainRefreshed>,
    ) {
        let Some(submit_info) = render_manager.submit_info.take() else {
            println!("No submit info for frame.");
            return;
        };
        let Some(swapchain_image_index) = render_manager.swapchain_image_index.take() else {
            println!("No swapchain image index for frame.");
//            swapchain.refresh(&vulkan, &Swapchain::create_info(window.width(), window.height())); 
 //           swapchain_refreshed.0 = true;
            return;
        };

        let current_frame = &mut render_manager.frame_resources[frame_index.index()];

        current_frame
            .command_pool
            .get_mut(current_frame.main_command_buffer)
            .unwrap()
            .end();

        let blit_command_buffer = current_frame
            .command_pool
            .get_mut(current_frame.blit_command_buffer)
            .unwrap();

        blit_command_buffer.begin();

        let submit_image = resource_manager
            .get_image(&submit_info.submit_image)
            .expect("Tried to submit a frame with an image that doesn't exist.");

        blit_command_buffer.pipeline_barrier(
            submit_info.last_stage,
            vk::PipelineStageFlags::TRANSFER,
            vec![
                ImageMemoryBarrier {
                    src_access_mask: submit_info.submit_image_last_access,
                    dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                    old_layout: submit_info.submit_image_last_layout,
                    new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image: submit_image,
                },
                ImageMemoryBarrier {
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    image: swapchain.image(swapchain_image_index),
                },
            ],
        );

        blit_command_buffer.blit_image(BlitImageInfo::with_defaults(
            submit_image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            swapchain.image(swapchain_image_index),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        ));

        blit_command_buffer.pipeline_barrier(vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::BOTTOM_OF_PIPE, vec![ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            image: swapchain.image(swapchain_image_index),
        }]);

        blit_command_buffer.end();

        default_executor.submit(QueueExecutorSubmitInfo {
            command_buffers: current_frame
                .command_pool
                .get_multiple_mut(vec![current_frame.main_command_buffer]),
            frame_index: frame_index.index(),
            wait_semaphores: vec![],
            signal_semaphores: vec![&current_frame.render_finished_semaphore],
            fence: None,
        });

        default_executor.submit(QueueExecutorSubmitInfo {
            command_buffers: current_frame
                .command_pool
                .get_multiple_mut(vec![current_frame.blit_command_buffer]),
            frame_index: frame_index.index(),
            wait_semaphores: vec![
                (
                    &current_frame.image_available_semaphore,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (
                    &current_frame.render_finished_semaphore,
                    submit_info.last_stage,
                ),
            ],
            signal_semaphores: vec![&current_frame.ready_to_present_semaphore],
            fence: Some(&current_frame.fence),
        });

        default_executor.present(
            &swapchain,
            swapchain_image_index,
            vec![&current_frame.ready_to_present_semaphore],
        );

        frame_index.next();
        render_manager.submit_info = None;
        render_manager.swapchain_image_index = None;
    }

    pub fn set_submit_info(&mut self, info: &FrameSubmitInfo) {
        self.submit_info = Some(info.clone());
    }
}

impl Drop for RenderManager {
    fn drop(&mut self) {
        for frame_resources in self.frame_resources.iter_mut() {
            frame_resources.fence.wait();
        }
    }
}
