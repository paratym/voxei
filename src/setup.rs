use crate::{
    constants,
    engine::{
        assets::{
            asset::Assets,
            loader::{obj::ObjLoader, octree::OctreeLoader, spirv::SpirVLoader},
            watched_shaders::WatchedShaders,
        },
        common::{camera::PrimaryCamera, time::Time},
        graphics::{
            device::DeviceResource,
            render_manager::{self, RenderManager},
            swapchain::SwapchainResource,
        },
        input::Input,
        window::window::{Window, WindowConfig},
    },
    settings::Settings,
};

pub fn setup_resources(app: &mut crate::app::App) {
    // Engine Resources
    app.resource_bank_mut().insert(Settings::default());
    app.resource_bank_mut().insert(Input::new());
    app.resource_bank_mut().insert(Time::new());

    let mut assets = Assets::new();
    assets.add_loader::<SpirVLoader>();
    assets.add_loader::<ObjLoader>();
    assets.add_loader::<OctreeLoader>();

    let mut watched_shaders = WatchedShaders::new();

    let window = Window::new(
        &WindowConfig {
            title: constants::WINDOW_TITLE.to_owned(),
            ..Default::default()
        },
        app.event_loop(),
    );
    let mut device_resource = DeviceResource::new(&window);
    let swapchain_resource = SwapchainResource::new(&mut device_resource, &window);
    let render_manager = RenderManager::new(&mut assets, &mut watched_shaders);
    let primary_camera = PrimaryCamera::new(&mut device_resource);

    app.resource_bank_mut().insert(window);
    app.resource_bank_mut().insert(assets);
    app.resource_bank_mut().insert(watched_shaders);
    app.resource_bank_mut().insert(device_resource);
    app.resource_bank_mut().insert(swapchain_resource);
    app.resource_bank_mut().insert(render_manager);
    app.resource_bank_mut().insert(primary_camera);
}
