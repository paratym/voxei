use voxei_macros::Resource;

use crate::{
    constants,
    engine::graphics::vulkan::{executor::QueueExecutor, vulkan::Vulkan},
};

#[derive(Resource)]
pub struct DefaultQueueExecutor {
    executor: QueueExecutor<{ constants::FRAMES_IN_FLIGHT }>,
}

impl DefaultQueueExecutor {
    pub fn new(vulkan: &Vulkan) -> Self {
        Self {
            executor: QueueExecutor::new(vulkan, constants::VULKAN_DEFAULT_QUEUE),
        }
    }

    pub fn executor(&self) -> &QueueExecutor<{ constants::FRAMES_IN_FLIGHT }> {
        &self.executor
    }
}
