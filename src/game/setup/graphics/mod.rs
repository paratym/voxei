use ash::vk;

use crate::engine::assets::asset::Assets;
use crate::engine::assets::watched_shaders::WatchedShaders;
use crate::engine::graphics::queues::DefaultQueueExecutor;
use crate::engine::graphics::render_manager::{FrameIndex, RenderManager};
use crate::engine::graphics::resource_manager::RenderResourceManager;
use crate::engine::graphics::vulkan::allocator::VulkanMemoryAllocator;
use crate::engine::graphics::vulkan::swapchain::SwapchainCreateInfo;
use crate::engine::window::window::{Window, WindowConfig};
use crate::game::app::App;

use crate::game::graphics::pipeline::voxel::pass::VoxelRenderPass;
use crate::{
    constants,
    engine::graphics::vulkan::{
        swapchain::Swapchain,
        vulkan::{
            QueueCapability, QueueConfig, QueuePriority, QueueResolution, SwapchainSupport, Vulkan,
            VulkanConfig,
        },
    },
};

pub fn setup_graphical_resources(app: &mut App) {
    let window = Window::new(
        &WindowConfig {
            title: constants::WINDOW_TITLE.to_owned(),
            ..Default::default()
        },
        app.event_loop(),
    );

    let vulkan = Vulkan::new(VulkanConfig {
        queues: vec![QueueConfig {
            name: constants::VULKAN_DEFAULT_QUEUE.to_owned(),
            capabilities: vec![
                QueueCapability::Graphics,
                QueueCapability::Transfer,
                QueueCapability::Present,
            ],
            priority: QueuePriority::Exclusive,
            resolution: QueueResolution::Panic,
        }],
        enable_validation: true,
        swapchain_support: SwapchainSupport::Supported(&window, &window),
    });

    let vulkan_memory_allocator = VulkanMemoryAllocator::new(&vulkan);

    let mut swapchain = Swapchain::new();
    swapchain.refresh(
        &vulkan,
        &SwapchainCreateInfo {
            width: window.width(),
            height: window.height(),
            create_image_views: false,
            image_usage: vk::ImageUsageFlags::TRANSFER_DST,
            preferred_image_count: 2,
            preferred_present_mode: vk::PresentModeKHR::FIFO,
        },
    );

    let default_queue_executor = DefaultQueueExecutor::new(&vulkan);

    let frame_index = FrameIndex::new();
    let resource_manager = RenderResourceManager::new(&vulkan);
    let render_manager = RenderManager::new(&vulkan);
    let voxel_render_pass = VoxelRenderPass::new(
        &mut app.resource_bank().get_resource_mut::<WatchedShaders>(),
        &mut app.resource_bank().get_resource_mut::<Assets>(),
        &vulkan,
    );

    app.resource_bank_mut().insert(window);
    app.resource_bank_mut().insert(vulkan);
    app.resource_bank_mut().insert(vulkan_memory_allocator);
    app.resource_bank_mut().insert(swapchain);
    app.resource_bank_mut().insert(default_queue_executor);
    app.resource_bank_mut().insert(frame_index);
    app.resource_bank_mut().insert(resource_manager);
    app.resource_bank_mut().insert(render_manager);
    app.resource_bank_mut().insert(voxel_render_pass);
}
