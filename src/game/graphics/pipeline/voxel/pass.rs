use voxei_macros::Resource;

use crate::engine::{
    assets::manager::Assets,
    graphics::vulkan::{
        objects::{
            compute::{ComputePipeline, ComputePipelineCreateInfo},
            pipeline_layout::PipelineLayoutCreateInfo,
            shader::Shader,
        },
        vulkan::Vulkan,
    },
};

#[derive(Resource)]
pub struct VoxelRenderPass {
    compute_pipeline: ComputePipeline,
}

impl VoxelRenderPass {
    pub fn new(vulkan: &Vulkan, assets: &mut Assets) -> Self {
        let shader = Shader::new(vulkan, &[]);

        let compute_pipeline = ComputePipeline::new(
            vulkan,
            ComputePipelineCreateInfo {
                shader: &shader,
                shader_entry_point: String::from("main"),
                pipeline_layout_info: PipelineLayoutCreateInfo {
                    descriptor_set_layouts: vec![],
                    push_constant_ranges: vec![],
                },
            },
        );
        Self { compute_pipeline }
    }
}
