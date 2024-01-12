use crate::{
    engine::{input::Input, system::System},
    game::graphics::render_manager::RenderManager,
};

use super::app::App;

pub fn game_loop(app: &mut App) {
    execute_system(app, RenderManager::render_frame);
    execute_system(app, Input::clear_inputs);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
