use std::collections::HashMap;

use voxei_macros::Resource;

use super::vulkan::{
    allocator::VulkanMemoryAllocator,
    objects::image::{OwnedImage, OwnedImageCreateInfo},
    vulkan::Vulkan,
};

#[derive(Resource)]
pub struct RenderResourceManager {
    images: HashMap<String, OwnedImage>,
}

impl RenderResourceManager {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    pub fn get_or_create_image(
        &mut self,
        vulkan: &Vulkan,
        vulkan_memory_allocator: &mut VulkanMemoryAllocator,
        name: &str,
        info: &OwnedImageCreateInfo,
    ) -> &OwnedImage {
        self.images.entry(name.to_string()).or_insert_with(|| {
            let image = OwnedImage::new(vulkan, vulkan_memory_allocator, info);
            image
        })
    }

    pub fn get_image(&self, name: &str) -> Option<&OwnedImage> {
        self.images.get(name)
    }
}
