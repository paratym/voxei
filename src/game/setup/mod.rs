use super::app::App;

mod graphics;

pub fn setup_resources(app: &mut App) {
    graphics::setup_graphical_resources(app);
}
