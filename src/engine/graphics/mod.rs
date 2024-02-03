use voxei_macros::Resource;

use super::resource::ResMut;

pub mod pass;
pub mod queues;
pub mod render_manager;
pub mod resource_manager;
pub mod shader_manager;
pub mod vulkan;

#[derive(Resource)]
pub struct SwapchainRefreshed(pub bool);

impl SwapchainRefreshed {
    pub fn clear(mut swapchain_refreshed: ResMut<SwapchainRefreshed>) {
        swapchain_refreshed.0 = false;
    }
}
