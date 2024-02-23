use crate::{
    constants,
    engine::{
        assets::{
            asset::Assets,
            loader::{obj::ObjLoader, octree::OctreeLoader, spirv::SpirVLoader},
            watched_shaders::WatchedShaders,
        },
        common::time::Time,
        graphics::{device::DeviceResource, swapchain::SwapchainResource},
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

    app.resource_bank_mut().insert({
        let mut assets = Assets::new();
        assets.add_loader::<SpirVLoader>();
        assets.add_loader::<ObjLoader>();
        assets.add_loader::<OctreeLoader>();

        assets
    });

    let window = Window::new(
        &WindowConfig {
            title: constants::WINDOW_TITLE.to_owned(),
            ..Default::default()
        },
        app.event_loop(),
    );
    let mut device_resource = DeviceResource::new(&window);
    let swapchain_resource = SwapchainResource::new(&mut device_resource, &window);

    app.resource_bank_mut().insert(window);
    app.resource_bank_mut().insert(device_resource);
    app.resource_bank_mut().insert(swapchain_resource);
    app.resource_bank_mut().insert(WatchedShaders::new());
}
