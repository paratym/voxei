use ash::vk;
use voxei_macros::Resource;

use crate::{
    constants,
    engine::{
        graphics::vulkan::{
            executor::QueueExecutorSubmitInfo,
            objects::{CommandBufferHandle, CommandPool, Fence, ImageMemoryBarrier, Semaphore},
            swapchain::Swapchain,
            vulkan::Vulkan,
        },
        resource::ResMut,
    },
};

use super::queues::DefaultQueueExecutor;

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
    image_available_semaphore: Semaphore,
    ready_to_present_semaphore: Semaphore,
    command_pool: CommandPool,
    main_command_buffer: CommandBufferHandle,
}

#[derive(Resource)]
pub struct RenderManager {
    frame_resources: [FrameResources; constants::FRAMES_IN_FLIGHT],
}

impl RenderManager {
    pub fn new(vulkan: &Vulkan) -> Self {
        let frame_resources = (0..constants::FRAMES_IN_FLIGHT)
            .map(|_| {
                let fence = Fence::new(vulkan, true);
                let image_available_semaphore = Semaphore::new(vulkan);
                let ready_to_present_semaphore = Semaphore::new(vulkan);

                let mut command_pool = CommandPool::new(vulkan);
                let [main_command_buffer] = command_pool.allocate();

                FrameResources {
                    fence,
                    image_available_semaphore,
                    ready_to_present_semaphore,
                    command_pool,
                    main_command_buffer,
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to create frames in flight."));

        Self { frame_resources }
    }

    pub fn render_frame(
        mut render_manager: ResMut<RenderManager>,
        mut frame_index: ResMut<FrameIndex>,
        mut default_executor: ResMut<DefaultQueueExecutor>,
        swapchain: ResMut<Swapchain>,
    ) {
        let current_frame = &mut render_manager.frame_resources[frame_index.index()];

        current_frame.fence.wait();

        let swapchain_image_index = swapchain
            .get_next_image_index(&current_frame.image_available_semaphore)
            .unwrap();

        current_frame.fence.reset();
        default_executor.release_frame_resources(frame_index.index());

        current_frame.command_pool.reset();
        let main_command_buffer = current_frame
            .command_pool
            .get_mut(current_frame.main_command_buffer)
            .unwrap();

        main_command_buffer.begin();

        main_command_buffer.pipeline_barrier(
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vec![ImageMemoryBarrier {
                image: swapchain.image(swapchain_image_index as usize),
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::MEMORY_READ,
            }],
        );

        main_command_buffer.end();

        default_executor.submit(QueueExecutorSubmitInfo {
            command_buffers: current_frame
                .command_pool
                .get_multiple_mut(vec![current_frame.main_command_buffer]),
            frame_index: frame_index.index(),
            wait_semaphores: vec![(
                &current_frame.image_available_semaphore,
                vk::PipelineStageFlags::TRANSFER,
            )],
            signal_semaphores: vec![&current_frame.ready_to_present_semaphore],
            fence: Some(&current_frame.fence),
        });

        default_executor.present(
            &swapchain,
            swapchain_image_index,
            vec![&current_frame.ready_to_present_semaphore],
        );

        frame_index.next();
    }
}

impl Drop for RenderManager {
    fn drop(&mut self) {
        for frame_resources in self.frame_resources.iter_mut() {
            frame_resources.fence.wait();
        }
    }
}
