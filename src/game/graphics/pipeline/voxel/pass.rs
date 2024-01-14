use ash::vk;
use voxei_macros::Resource;

use crate::{
    engine::{
        assets::{
            asset::Assets,
            watched_shaders::{DependencySignal, WatchedShaders},
        },
        graphics::{
            render_manager::{FrameIndex, RenderManager},
            resource_manager::RenderResourceManager,
            vulkan::{
                allocator::VulkanMemoryAllocator,
                objects::{
                    compute::{ComputePipeline, ComputePipelineCreateInfo},
                    descriptor_set::{DescriptorSetLayout, DescriptorSetWriteStorageImageInfo},
                    image::ImageMemoryBarrier,
                    pipeline_layout::PipelineLayoutCreateInfo,
                    shader::Shader,
                },
                swapchain::Swapchain,
                vulkan::Vulkan,
            },
        },
        resource::{Res, ResMut},
    },
    game::graphics::{gfx_constants, pipeline::util as pipeline_util},
};

#[derive(Resource)]
pub struct VoxelRenderPass {
    compute_pipeline: Option<ComputePipeline>,
    shader_signal: DependencySignal,
    descriptor_set_layout: DescriptorSetLayout,
}

impl VoxelRenderPass {
    pub fn new(watched_shaders: &mut WatchedShaders, assets: &mut Assets, vulkan: &Vulkan) -> Self {
        let shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            gfx_constants::VOXEL_COMPUTE_SHADER_PATH,
            gfx_constants::VOXEL_COMPUTE_SHADER_NAME,
            &shader_signal,
        );

        let mut descriptor_set_layout = DescriptorSetLayout::builder();
        descriptor_set_layout.add_binding(
            0,
            vk::DescriptorType::STORAGE_IMAGE,
            1,
            vk::ShaderStageFlags::COMPUTE,
        );
        let descriptor_set_layout = descriptor_set_layout.build(vulkan);

        Self {
            compute_pipeline: None,
            shader_signal,
            descriptor_set_layout,
        }
    }

    pub fn update(
        mut voxel_pass: ResMut<VoxelRenderPass>,
        watched_shaders: ResMut<WatchedShaders>,
        vulkan: Res<Vulkan>,
        render_resource_manager: ResMut<RenderResourceManager>,
        frame_index: Res<FrameIndex>,
    ) {
        if watched_shaders.is_dependency_signaled(&voxel_pass.shader_signal) {
            let shader_code = watched_shaders
                .get_shader(gfx_constants::VOXEL_COMPUTE_SHADER_NAME)
                .expect("Voxel compute shader not loaded but we thought it was.");
            let shader = Shader::new(&vulkan, shader_code.as_slice());
            let compute_pipeline = ComputePipeline::new(
                &vulkan,
                ComputePipelineCreateInfo {
                    shader: &shader,
                    shader_entry_point: "main".to_owned(),
                    pipeline_layout_info: PipelineLayoutCreateInfo {
                        descriptor_set_layouts: vec![voxel_pass.descriptor_set_layout()],
                        push_constant_ranges: vec![],
                    },
                },
            );

            voxel_pass.compute_pipeline = Some(compute_pipeline);
        }

        // Write descriptor sets (creation is handled in pipeline_util::refresh_render_resources)
        let backbuffer_image = pipeline_util::get_backbuffer_image(&render_resource_manager);
        let mut descriptor_set = render_resource_manager
            .get_descriptor_set_mut(
                gfx_constants::VOXEL_DESCRIPTOR_SET_NAME,
                frame_index.index(),
            )
            .unwrap();
        descriptor_set
            .writer(&vulkan)
            .write_storage_image(0, backbuffer_image, vk::ImageLayout::GENERAL)
            .submit_writes();
    }

    pub fn render(
        voxel_pass: Res<VoxelRenderPass>,
        mut render_manager: ResMut<RenderManager>,
        frame_index: Res<FrameIndex>,
        render_resources: ResMut<RenderResourceManager>,
    ) {
        let main_command_buffer = render_manager.main_command_buffer(frame_index.index());
        let backbuffer_image = pipeline_util::get_backbuffer_image(&render_resources);

        main_command_buffer.pipeline_barrier(
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vec![ImageMemoryBarrier {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::GENERAL,
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::SHADER_WRITE,
                image: backbuffer_image,
            }],
        );

        main_command_buffer.bind_compute_pipeline(voxel_pass.compute_pipeline());

        let descriptor_pool = render_resources.get_descriptor_pool();
        main_command_buffer.bind_descriptor_sets(
            voxel_pass.compute_pipeline().instance().pipeline_layout(),
            vk::PipelineBindPoint::COMPUTE,
            vec![(
                &descriptor_pool,
                vec![render_resources
                    .get_descriptor_set_handle(
                        gfx_constants::VOXEL_DESCRIPTOR_SET_NAME,
                        frame_index.index(),
                    )
                    .unwrap()],
            )],
        );

        main_command_buffer.dispatch(
            backbuffer_image.instance().info().width() / 16,
            backbuffer_image.instance().info().height() / 16,
            1,
        );
    }

    pub fn compute_pipeline(&self) -> &ComputePipeline {
        self.compute_pipeline
            .as_ref()
            .expect("Voxel compute pipeline not loaded but we thought it was.")
    }

    pub fn descriptor_set_layout(&self) -> &DescriptorSetLayout {
        &self.descriptor_set_layout
    }
}
