use std::time::Instant;

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
    let time = Instant::now();
    execute_system(app, Time::update);

    execute_system(app, Assets::update);
    execute_system(app, WatchedShaders::update);

    // Physics
    execute_system(app, update_player_controller);

    // Update voxel world
    execute_system(app, VoxelWorld::update_world_position);
    execute_system(app, VoxelWorld::update_world_streaming);

    // Update GPU non-buffer resources
    execute_system(app, PipelineManager::update);
    execute_system(app, RenderManager::update);

    // Update render resources
    execute_system(app, Camera::update_cameras);
    execute_system(app, VoxelPipeline::update_world_changes);

    // Render
    execute_system(app, RenderManager::render);

    // Post render clear
    execute_system(app, Input::clear_inputs);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
