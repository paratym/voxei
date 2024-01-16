use crate::{
    engine::{
        assets::{asset::Assets, loader::spirv::SpirVLoader, watched_shaders::WatchedShaders},
        input::Input,
    },
    settings::Settings,
};

use super::app::App;

mod graphics;

pub fn setup_resources(app: &mut App) {
    app.resource_bank_mut().insert(Settings::default());
    app.resource_bank_mut().insert(Input::new());

    // Assets
    let mut assets = Assets::new();
    assets.add_loader::<SpirVLoader>();
    app.resource_bank_mut().insert(assets);

    // Graphics
    app.resource_bank_mut().insert(WatchedShaders::new());
    graphics::setup_graphical_resources(app);
}
