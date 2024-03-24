use crate::{
    constants,
    engine::{
        assets::{
            asset::Assets,
            loader::{obj::ObjLoader, spirv::SpirVLoader},
            watched_shaders::WatchedShaders,
        },
        common::time::Time,
        ecs::ecs_world::ECSWorld,
        graphics::{
            device::DeviceResource,
            pass::voxel::VoxelPipeline,
            pipeline_manager::{self, PipelineManager},
            render_manager::{self, RenderManager},
            swapchain::SwapchainResource,
        },
        input::Input,
        voxel::vox_world::VoxelWorld,
        window::window::{Window, WindowConfig},
    },
    game::player::player::spawn_player,
    settings::Settings,
};

pub fn setup_resources(app: &mut crate::app::App) {
    // Engine Resources
    app.resource_bank_mut().insert(Settings::default());
    app.resource_bank_mut().insert(Input::new());
    app.resource_bank_mut().insert(Time::new());

    let mut ecs_world = ECSWorld::new();
    let vox_world = VoxelWorld::new();

    let mut assets = Assets::new();
    assets.add_loader::<SpirVLoader>();
    assets.add_loader::<ObjLoader>();

    let mut watched_shaders = WatchedShaders::new();
    let mut pipeline_manager = PipelineManager::new();

    let window = Window::new(
        &WindowConfig {
            title: constants::WINDOW_TITLE.to_owned(),
            ..Default::default()
        },
        app.event_loop(),
    );
    let mut device_resource = DeviceResource::new(&window);
    let swapchain_resource = SwapchainResource::new(&mut device_resource, &window);
    let render_manager = RenderManager::new();
    let voxel_pipeline = VoxelPipeline::new(
        &mut assets,
        &mut watched_shaders,
        &mut pipeline_manager,
        &mut device_resource,
    );

    // ECS World spawns
    spawn_player(&mut ecs_world, &mut device_resource);

    app.resource_bank_mut().insert(window);
    app.resource_bank_mut().insert(assets);
    app.resource_bank_mut().insert(ecs_world);
    app.resource_bank_mut().insert(vox_world);
    app.resource_bank_mut().insert(watched_shaders);
    app.resource_bank_mut().insert(device_resource);
    app.resource_bank_mut().insert(swapchain_resource);
    app.resource_bank_mut().insert(render_manager);
    app.resource_bank_mut().insert(pipeline_manager);
    app.resource_bank_mut().insert(voxel_pipeline);
}
