use crate::{
    engine::{
        assets::{
            asset::Assets,
            loader::{obj::ObjLoader, spirv::SpirVLoader},
            watched_shaders::WatchedShaders,
        },
        common::time::Time,
        input::Input,
    },
    settings::Settings,
};

use super::app::App;

mod graphics;
mod world;

pub fn setup_resources(app: &mut App) {
    app.resource_bank_mut().insert(Settings::default());
    app.resource_bank_mut().insert(Input::new());
    app.resource_bank_mut().insert(Time::new());

    // Assets
    let mut assets = Assets::new();
    assets.add_loader::<SpirVLoader>();
    assets.add_loader::<ObjLoader>();
    app.resource_bank_mut().insert(assets);

    // Graphics
    app.resource_bank_mut().insert(WatchedShaders::new());
    graphics::setup_graphical_resources(app);

    // World
    world::setup_world_resources(app);
}
