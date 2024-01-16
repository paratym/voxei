use crate::engine::{
    assets::{asset::Assets, watched_shaders::WatchedShaders},
    common::camera::PrimaryCamera,
    graphics::render_manager::RenderManager,
    input::Input,
    system::System,
};

use super::{
    app::App,
    graphics::{
        self,
        pipeline::{util as pipeline_util, voxel::pass::VoxelRenderPass},
    },
};

pub fn game_loop(app: &mut App) {
    // Asset systems updating
    execute_system(app, Assets::update);
    execute_system(app, WatchedShaders::update);

    execute_system(app, PrimaryCamera::update);

    // Rendering
    execute_system(app, RenderManager::begin_frame);

    // Update render systems, any changes deps (swapchain, assets, lod, etc)
    execute_system(app, pipeline_util::refresh_render_resources);
    execute_system(app, VoxelRenderPass::update);

    // Draw
    execute_system(app, VoxelRenderPass::render);
    execute_system(app, graphics::set_submit_info);
    execute_system(app, RenderManager::submit_frame);

    execute_system(app, Input::clear_inputs);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
