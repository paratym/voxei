use ash::vk;

use crate::engine::{
    graphics::render_manager::{FrameSubmitInfo, RenderManager},
    resource::ResMut,
};

mod gfx_constants;
pub mod pipeline;

pub fn set_submit_info(mut render_manager: ResMut<RenderManager>) {
    render_manager.set_submit_info(&FrameSubmitInfo {
        submit_image: gfx_constants::BACKBUFFER_IMAGE.to_string(),
        submit_image_last_layout: vk::ImageLayout::GENERAL,
        submit_image_last_access: vk::AccessFlags::SHADER_WRITE,
        last_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
    });
}
