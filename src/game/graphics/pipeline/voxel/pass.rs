use voxei_macros::Resource;

use crate::engine::{
    assets::asset::Assets,
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
    compute_pipeline: Option<ComputePipeline>,
}

impl VoxelRenderPass {
    pub fn new(vulkan: &Vulkan, assets: &mut Assets) -> Self {
        Self {
            compute_pipeline: None,
        }
    }

    pub fn update(&mut self, vulkan: &Vulkan, assets: &mut Assets) {
        if self.compute_pipeline.is_none() {
            let shader = Shader::new(vulkan, &[]);

            self.compute_pipeline = Some(ComputePipeline::new(
                vulkan,
                ComputePipelineCreateInfo {
                    shader: &shader,
                    shader_entry_point: String::from("main"),
                    pipeline_layout_info: PipelineLayoutCreateInfo {
                        descriptor_set_layouts: vec![],
                        push_constant_ranges: vec![],
                    },
                },
            ));
        }
    }
}
