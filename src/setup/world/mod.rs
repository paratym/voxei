use crate::{
    engine::graphics::vulkan::{allocator::VulkanMemoryAllocator, vulkan::Vulkan},
    game::{
        app::App,
        world::{sponza::Sponza, terrain::Terrain},
    },
};

pub fn setup_world_resources(app: &mut App) {
    let sponza = Sponza::new();

    app.resource_bank_mut().insert(sponza);
}
