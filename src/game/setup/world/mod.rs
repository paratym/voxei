use crate::{
    engine::graphics::vulkan::{allocator::VulkanMemoryAllocator, vulkan::Vulkan},
    game::{
        app::App,
        world::{sponza::Sponza, terrain::Terrain},
    },
};

pub fn setup_world_resources(app: &mut App) {
    let terrain = Terrain::new(
        &app.resource_bank().get_resource::<Vulkan>(),
        &mut app
            .resource_bank()
            .get_resource_mut::<VulkanMemoryAllocator>(),
    );

    let sponza = Sponza::new();

    app.resource_bank_mut().insert(terrain);
    app.resource_bank_mut().insert(sponza);
}
