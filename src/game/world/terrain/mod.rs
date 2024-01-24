use voxei_macros::Resource;

use crate::engine::{
    graphics::vulkan::{allocator::VulkanMemoryAllocator, vulkan::Vulkan},
    resource::ResMut,
};

pub mod chunk;

#[derive(Resource)]
pub struct Terrain {}

impl Terrain {
    pub fn new(vulkan: &Vulkan, vulkan_memory_allocator: &mut VulkanMemoryAllocator) -> Self {
        Self {}
    }

    pub fn update(terrain: ResMut<Terrain>) {}
}
