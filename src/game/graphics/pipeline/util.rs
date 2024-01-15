use ash::vk;

use crate::{
    engine::{
        graphics::{
            resource_manager::RenderResourceManager,
            vulkan::{
                allocator::VulkanMemoryAllocator,
                objects::image::{util::ImageViewCreateInfo, Image, OwnedImageCreateInfo},
                swapchain::Swapchain,
                vulkan::Vulkan,
            },
        },
        resource::{Res, ResMut},
    },
    game::graphics::gfx_constants,
};

use super::voxel::pass::VoxelRenderPass;

pub fn refresh_render_resources(
    vulkan: Res<Vulkan>,
    mut vulkan_memory_allocator: ResMut<VulkanMemoryAllocator>,
    swapchain: Res<Swapchain>,
    mut render_resource_manager: ResMut<RenderResourceManager>,
    voxel_pass: Res<VoxelRenderPass>,
) {
    let has_initialized = render_resource_manager.has_image(gfx_constants::BACKBUFFER_IMAGE_NAME);

    if !has_initialized {
        create_backbuffer_image(
            &vulkan,
            &mut vulkan_memory_allocator,
            &swapchain,
            &mut render_resource_manager,
        );

        // Voxel Compute Descriptor Set
        render_resource_manager.create_descriptor_sets(
            gfx_constants::VOXEL_DESCRIPTOR_SET_NAME,
            &voxel_pass.descriptor_set_layout(),
        );
    }
}

pub fn create_backbuffer_image<'a>(
    vulkan: &Vulkan,
    vulkan_memory_allocator: &mut VulkanMemoryAllocator,
    swapchain: &Swapchain,
    render_resource_manager: &'a mut RenderResourceManager,
) {
    render_resource_manager.create_image(
        vulkan,
        vulkan_memory_allocator,
        gfx_constants::BACKBUFFER_IMAGE_NAME,
        &OwnedImageCreateInfo {
            width: swapchain.instance().info().extent().width,
            height: swapchain.instance().info().extent().height,
            usage: vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            samples: vk::SampleCountFlags::TYPE_1,
            view_create_info: Some(ImageViewCreateInfo {
                view_type: vk::ImageViewType::TYPE_2D,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            }),
        },
    )
}

pub fn get_backbuffer_image<'a>(
    render_resource_manager: &'a RenderResourceManager,
) -> &'a dyn Image {
    render_resource_manager
        .get_image(gfx_constants::BACKBUFFER_IMAGE_NAME)
        .expect("Backbuffer image not created.")
}
