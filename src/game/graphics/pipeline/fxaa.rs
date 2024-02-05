use ash::vk;
use voxei_macros::Resource;

use crate::constants;
use crate::engine::assets::asset::Assets;
use crate::engine::assets::watched_shaders::WatchedShaders;
use crate::engine::graphics::render_manager::{self, FrameIndex, RenderManager};
use crate::engine::graphics::resource_manager::RenderResourceManager;
use crate::engine::graphics::vulkan::allocator::VulkanMemoryAllocator;
use crate::engine::graphics::vulkan::objects::buffer::BufferCreateInfo;
use crate::engine::graphics::vulkan::objects::compute::ComputePipelineCreateInfo;
use crate::engine::graphics::vulkan::objects::glsl::GlslVec2f;
use crate::engine::graphics::vulkan::objects::image::ImageMemoryBarrier;
use crate::engine::graphics::vulkan::objects::pipeline_layout::PipelineLayoutCreateInfo;
use crate::engine::graphics::vulkan::objects::shader::Shader;
use crate::engine::graphics::vulkan::vulkan::Vulkan;
use crate::engine::graphics::SwapchainRefreshed;
use crate::engine::resource::{Res, ResMut};
use crate::engine::{
    assets::watched_shaders::DependencySignal,
    graphics::vulkan::objects::{
        buffer::Buffer, compute::ComputePipeline, descriptor_set::DescriptorSetLayout,
    },
};
use crate::game::graphics::gfx_constants;
use crate::game::graphics::pipeline::util as pipeline_util;

use super::voxel::pass::{BufferData, VoxelRenderPass};

#[derive(Resource)]
pub struct FxaaPass {
    compute_pipeline: Option<ComputePipeline>,
    shader_signal: DependencySignal,
    descriptor_set_layout: DescriptorSetLayout,
    main_uniform_buffers: [Buffer; constants::FRAMES_IN_FLIGHT],
}

impl FxaaPass {
    pub fn new(
        watched_shaders: &mut WatchedShaders,
        assets: &mut Assets,
        vulkan: &Vulkan,
        vulkan_memory_allocator: &mut VulkanMemoryAllocator,
    ) -> Self {
        let shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            gfx_constants::FXAA_COMPUTE_SHADER_PATH,
            gfx_constants::FXAA_COMPUTE_SHADER_NAME,
            &shader_signal,
        );

        let mut descriptor_set_layout = DescriptorSetLayout::builder();
        descriptor_set_layout.add_binding(
            0,
            vk::DescriptorType::STORAGE_IMAGE,
            1,
            vk::ShaderStageFlags::COMPUTE,
        );
        descriptor_set_layout.add_binding(
            1,
            vk::DescriptorType::STORAGE_IMAGE,
            1,
            vk::ShaderStageFlags::COMPUTE,
        );
        descriptor_set_layout.add_binding(
            2,
            vk::DescriptorType::UNIFORM_BUFFER,
            1,
            vk::ShaderStageFlags::COMPUTE,
        );
        let descriptor_set_layout = descriptor_set_layout.build(vulkan);

        let main_uniform_buffers = (0..2)
            .map(|_| {
                Buffer::new(
                    vulkan,
                    vulkan_memory_allocator,
                    &BufferCreateInfo {
                        size: std::mem::size_of::<BufferData>() as u64,
                        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                        // TODO: implement staging
                        memory_usage: vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    },
                )
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Self {
            compute_pipeline: None,
            shader_signal,
            descriptor_set_layout,
            main_uniform_buffers,
        }
    }

    pub fn update(
        mut fxaa_pass: ResMut<FxaaPass>,
        watched_shaders: ResMut<WatchedShaders>,
        vulkan: Res<Vulkan>,
        render_resource_manager: ResMut<RenderResourceManager>,
        frame_index: Res<FrameIndex>,
        swapchain_refreshed: Res<SwapchainRefreshed>,
    ) {
        if watched_shaders.is_dependency_signaled(&fxaa_pass.shader_signal) || swapchain_refreshed.0
        {
            let shader_code = watched_shaders
                .get_shader(gfx_constants::FXAA_COMPUTE_SHADER_NAME)
                .unwrap();
            let shader = Shader::new(&vulkan, &shader_code);
            let compute_pipeline = ComputePipeline::new(
                &vulkan,
                ComputePipelineCreateInfo {
                    shader: &shader,
                    shader_entry_point: "main".to_owned(),
                    pipeline_layout_info: PipelineLayoutCreateInfo {
                        descriptor_set_layouts: vec![&fxaa_pass.descriptor_set_layout()],
                        push_constant_ranges: vec![],
                    },
                },
            );
            fxaa_pass.compute_pipeline = Some(compute_pipeline);
        }

        // Write descriptor sets, gosh this code is so bad ill refactor it and like the all the
        // buffers are also host coherent so theyre slow

        let backbuffer_image = pipeline_util::get_backbuffer_image(&render_resource_manager);
        let fxaa_output_image = pipeline_util::get_fxaa_output_image(&render_resource_manager);
        let mut descriptor_set = render_resource_manager
            .get_descriptor_set_mut(gfx_constants::FXAA_DESCRIPTOR_SET_NAME, frame_index.index())
            .unwrap();
        let d_writer = descriptor_set
            .writer(&vulkan)
            .write_storage_image(0, backbuffer_image, vk::ImageLayout::GENERAL)
            .write_storage_image(1, fxaa_output_image, vk::ImageLayout::GENERAL)
            .write_uniform_buffer(2, &fxaa_pass.main_uniform_buffers[frame_index.index()]);

        d_writer.submit_writes();

        // Update uniform buffers
        let map_ptr = fxaa_pass.main_uniform_buffers[frame_index.index()]
            .instance()
            .allocation()
            .instance()
            .map_memory(0);

        let window_extent = GlslVec2f::new(
            backbuffer_image.instance().info().width() as f32,
            backbuffer_image.instance().info().height() as f32,
        );

        unsafe {
            std::ptr::copy_nonoverlapping(
                &BufferData {
                    window_extent: window_extent.into(),
                },
                map_ptr as *mut BufferData,
                1,
            );
        }

        fxaa_pass.main_uniform_buffers[frame_index.index()]
            .instance()
            .allocation()
            .instance()
            .unmap_memory();
    }

    pub fn render(
        fxaa_pass: Res<FxaaPass>,
        voxel_pass: Res<VoxelRenderPass>,
        mut render_manager: ResMut<RenderManager>,
        frame_index: Res<FrameIndex>,
        render_resources: Res<RenderResourceManager>,
    ) {
        let main_command_buffer = render_manager.main_command_buffer(frame_index.index());
        let backbuffer_image = pipeline_util::get_backbuffer_image(&render_resources);
        let fxaa_output_image = pipeline_util::get_fxaa_output_image(&render_resources);

        let mut img_memory_barriers = vec![ImageMemoryBarrier {
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::GENERAL,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::SHADER_WRITE,
            image: fxaa_output_image,
        }];

        img_memory_barriers.push(ImageMemoryBarrier {
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::GENERAL,
            src_access_mask: vk::AccessFlags::SHADER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_WRITE,
            image: backbuffer_image,
        });

        main_command_buffer.pipeline_barrier(
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            img_memory_barriers,
        );

        if fxaa_pass.compute_pipeline.is_none() || !voxel_pass.wrote_sponza {
            return;
        }

        main_command_buffer.bind_compute_pipeline(fxaa_pass.compute_pipeline.as_ref().unwrap());

        let descriptor_pool = render_resources.get_descriptor_pool();
        main_command_buffer.bind_descriptor_sets(
            fxaa_pass
                .compute_pipeline
                .as_ref()
                .unwrap()
                .instance()
                .pipeline_layout(),
            vk::PipelineBindPoint::COMPUTE,
            vec![(
                &descriptor_pool,
                vec![render_resources
                    .get_descriptor_set_handle(
                        gfx_constants::FXAA_DESCRIPTOR_SET_NAME,
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

    pub fn descriptor_set_layout(&self) -> &DescriptorSetLayout {
        &self.descriptor_set_layout
    }
}
