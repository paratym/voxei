use crate::{
    app::App,
    engine::{
        assets::{asset::Assets, watched_shaders::WatchedShaders},
        common::{camera::Camera, time::Time},
        graphics::{
            pass::voxel::VoxelPipeline, pipeline_manager::PipelineManager,
            render_manager::RenderManager,
        },
        input::Input,
        system::System,
        voxel::vox_world::VoxelWorld,
    },
    game::player::player::update_player_controller,
};

pub fn game_loop(app: &mut App) {
    execute_system(app, Time::update);

    execute_system(app, Assets::update);
    execute_system(app, WatchedShaders::update);

    execute_system(app, update_player_controller);

    execute_system(app, Camera::update_cameras);
    execute_system(app, PipelineManager::update);
    execute_system(app, RenderManager::update);
    execute_system(app, RenderManager::render);

    execute_system(app, VoxelWorld::clear_changes);
    execute_system(app, Input::clear_inputs);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
