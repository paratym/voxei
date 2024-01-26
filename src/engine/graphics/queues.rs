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

impl std::ops::Deref for DefaultQueueExecutor {
    type Target = QueueExecutor<{ constants::FRAMES_IN_FLIGHT }>;

    fn deref(&self) -> &Self::Target {
        &self.executor
    }
}

impl std::ops::DerefMut for DefaultQueueExecutor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.executor
    }
}

impl Drop for DefaultQueueExecutor {
    fn drop(&mut self) {
        self.executor.wait_idle();
    }
}
