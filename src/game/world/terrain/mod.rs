use std::collections::HashMap;

use voxei_macros::Resource;

use crate::engine::{
    graphics::vulkan::{allocator::VulkanMemoryAllocator, vulkan::Vulkan},
    resource::ResMut,
};

pub mod chunk;

#[derive(Resource)]
pub struct Terrain {
    chunks: HashMap<(i32, i32), chunk::Chunk>,
}

impl Terrain {
    pub fn update(terrain: ResMut<Terrain>) {}
}
