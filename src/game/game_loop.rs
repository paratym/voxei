use crate::engine::{
    assets::{asset::Assets, watched_shaders::WatchedShaders},
    graphics::render_manager::RenderManager,
    input::Input,
    system::System,
};

use super::{app::App, graphics};

pub fn game_loop(app: &mut App) {
    execute_system(app, Assets::update);
    execute_system(app, WatchedShaders::update);
    execute_system(app, RenderManager::begin_frame);
    execute_system(app, graphics::set_submit_info);
    execute_system(app, RenderManager::submit_frame);
    execute_system(app, Input::clear_inputs);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
