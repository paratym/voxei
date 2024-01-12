use crate::engine::input::Input;

use super::app::App;

mod graphics;

pub fn setup_resources(app: &mut App) {
    app.resource_bank_mut().insert(Input::new());
    graphics::setup_graphical_resources(app);
}
