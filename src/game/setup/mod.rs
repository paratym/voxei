use crate::engine::{assets::manager::Assets, input::Input};

use super::app::App;

mod graphics;

pub fn setup_resources(app: &mut App) {
    app.resource_bank_mut().insert(Input::new());
    app.resource_bank_mut().insert(Assets::new());
    graphics::setup_graphical_resources(app);
}
