use crate::engine::{
    assets::{asset::Assets, watched_shaders::WatchedShaders},
    common::{camera::PrimaryCamera, time::Time},
    graphics::{render_manager::RenderManager, SwapchainRefreshed},
    input::Input,
    system::System,
};

use super::{
    app::App,
    graphics::{
        self,
        pipeline::{util as pipeline_util, voxel::pass::VoxelRenderPass},
    },
    world::sponza::Sponza,
};

pub fn game_loop(app: &mut App) {
    // Update time
    execute_system(app, Time::update);

    // Asset systems updating
    execute_system(app, Assets::update);
    execute_system(app, WatchedShaders::update);

    // Update camera position and gpu buffers
    execute_system(app, PrimaryCamera::update);

    // Update world resources
    execute_system(app, Sponza::update);

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
    execute_system(app, SwapchainRefreshed::clear);
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
