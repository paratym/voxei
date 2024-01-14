use std::sync::Arc;

use ash::vk::{self};
use slotmap::{new_key_type, SlotMap};
use voxei_macros::VulkanResource;

use crate::engine::graphics::vulkan::{
    util::WeakGenericResourceDep,
    vulkan::{Vulkan, VulkanDep},
};

use crate::engine::graphics::vulkan::util::VulkanResourceDep;

use super::{
    compute::ComputePipeline,
    descriptor_set::{DescriptorSetHandle, DescriptorSetPool},
    image::{Image, ImageMemoryBarrier},
    pipeline_layout::PipelineLayoutInstance,
};

new_key_type! { pub struct CommandBufferHandle; }

pub struct CommandBuffer {
    vulkan_dep: VulkanDep,
    command_pool: std::sync::Weak<CommandPoolInstance>,
    command_buffer: ash::vk::CommandBuffer,
    recorded_dependencies: Vec<WeakGenericResourceDep>,
}

impl CommandBuffer {
    pub fn begin(&mut self) {
        self.recorded_dependencies
            .push(self.command_pool.into_generic_weak());

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.vulkan_dep
                .device()
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin command buffer");
        }
    }

    pub fn end(&mut self) {
        unsafe {
            self.vulkan_dep
                .device()
                .end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer");
        }
    }

    pub fn blit_image(&mut self, info: util::BlitImageInfo) {
        self.recorded_dependencies
            .push(Arc::downgrade(&info.src_image.create_generic_dep()));

        unsafe {
            self.vulkan_dep.device().cmd_blit_image(
                self.command_buffer,
                info.src_image.instance().image(),
                info.src_image_layout,
                info.dst_image.instance().image(),
                info.dst_image_layout,
                &[vk::ImageBlit::from(&info)],
                info.scaling,
            );
        }
    }

    pub fn bind_compute_pipeline(&mut self, compute_pipeline: &ComputePipeline) {
        self.recorded_dependencies
            .push(compute_pipeline.create_dep().into_generic_weak());

        unsafe {
            self.vulkan_dep.device().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                compute_pipeline.instance().pipeline(),
            );
        }
    }

    pub fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayoutInstance,
        pipeline_bind_point: vk::PipelineBindPoint,
        descriptor_sets: Vec<(&DescriptorSetPool, Vec<DescriptorSetHandle>)>,
    ) {
        let descriptor_sets = descriptor_sets
            .into_iter()
            .flat_map(|(descriptor_pool, descriptor_set_handles)| {
                self.recorded_dependencies
                    .push(descriptor_pool.create_dep().into_generic_weak());

                descriptor_pool
                    .get_multiple(descriptor_set_handles)
                    .into_iter()
                    .map(|descriptor_set| descriptor_set.descriptor_set())
            })
            .collect::<Vec<_>>();

        unsafe {
            self.vulkan_dep.device().cmd_bind_descriptor_sets(
                self.command_buffer,
                pipeline_bind_point,
                pipeline_layout.layout(),
                0,
                &descriptor_sets,
                &[],
            );
        }
    }

    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        unsafe {
            self.vulkan_dep
                .device()
                .cmd_dispatch(self.command_buffer, x, y, z);
        }
    }

    pub fn pipeline_barrier(
        &mut self,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
        image_memory_barriers: Vec<ImageMemoryBarrier>,
    ) {
        self.recorded_dependencies
            .extend(image_memory_barriers.iter().map(|image_memory_barrier| {
                Arc::downgrade(&image_memory_barrier.image.create_generic_dep())
            }));
        let vk_image_memory_barriers = image_memory_barriers
            .into_iter()
            .map(|image_memory_barrier| image_memory_barrier.into())
            .collect::<Vec<_>>();

        unsafe {
            self.vulkan_dep.device().cmd_pipeline_barrier(
                self.command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &vk_image_memory_barriers,
            );
        }
    }

    pub fn clear_color_image(
        &mut self,
        image: &dyn Image,
        clear_color: vk::ClearColorValue,
        subresource_range: vk::ImageSubresourceRange,
    ) {
        self.recorded_dependencies
            .push(Arc::downgrade(&image.create_generic_dep()));

        unsafe {
            self.vulkan_dep.device().cmd_clear_color_image(
                self.command_buffer,
                image.instance().image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_color,
                &[subresource_range],
            );
        }
    }

    pub fn take_recorded_dependencies(&mut self) -> Vec<WeakGenericResourceDep> {
        std::mem::take(&mut self.recorded_dependencies)
    }

    pub fn command_buffer(&self) -> ash::vk::CommandBuffer {
        self.command_buffer
    }
}

pub type CommandPoolDep = Arc<CommandPoolInstance>;

#[derive(VulkanResource)]
pub struct CommandPoolInstance {
    vulkan_dep: VulkanDep,
    command_pool: ash::vk::CommandPool,
}

impl Drop for CommandPoolInstance {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_dep
                .device()
                .destroy_command_pool(self.command_pool, None);
        }
    }
}

pub struct CommandPool {
    instance: Arc<CommandPoolInstance>,
    command_buffers: SlotMap<CommandBufferHandle, CommandBuffer>,
}

impl CommandPool {
    pub fn new(vulkan: &Vulkan) -> Self {
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(vulkan.default_queue().queue_family_index());

        // Safety: The command pool is dropped when the internal command pool is dropped
        let command_pool = unsafe {
            vulkan
                .device()
                .create_command_pool(&command_pool_create_info, None)
                .expect("Failed to create command pool")
        };

        Self {
            instance: Arc::new(CommandPoolInstance {
                vulkan_dep: vulkan.create_dep(),
                command_pool,
            }),
            command_buffers: SlotMap::with_key(),
        }
    }

    pub fn get(&self, handle: CommandBufferHandle) -> Option<&CommandBuffer> {
        self.command_buffers.get(handle)
    }

    pub fn get_multiple(&self, handles: Vec<CommandBufferHandle>) -> Vec<&CommandBuffer> {
        self.command_buffers
            .iter()
            .filter(|(handle, _)| handles.iter().any(|h| h == handle))
            .map(|(_, command_buffer)| command_buffer)
            .collect()
    }

    pub fn get_mut(&mut self, handle: CommandBufferHandle) -> Option<&mut CommandBuffer> {
        self.command_buffers.get_mut(handle)
    }

    pub fn get_multiple_mut(
        &mut self,
        handles: Vec<CommandBufferHandle>,
    ) -> Vec<&mut CommandBuffer> {
        self.command_buffers
            .iter_mut()
            .filter(|(handle, _)| handles.iter().any(|h| h == handle))
            .map(|(_, command_buffer)| command_buffer)
            .collect()
    }

    pub fn reset(&mut self) {
        unsafe {
            self.instance
                .vulkan_dep
                .device()
                .reset_command_pool(
                    self.instance.command_pool,
                    vk::CommandPoolResetFlags::empty(),
                )
                .expect("Failed to reset command pool");
        }
    }

    pub fn allocate<const N: usize>(&mut self) -> [CommandBufferHandle; N] {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.instance.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(N as u32);

        let command_buffers = unsafe {
            self.instance
                .vulkan_dep
                .device()
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Failed to allocate command buffers")
        }
        .into_iter()
        .map(|command_buffer| CommandBuffer {
            vulkan_dep: self.instance.vulkan_dep.clone(),
            command_pool: Arc::downgrade(&self.instance),
            command_buffer,
            recorded_dependencies: Vec::new(),
        })
        .collect::<Vec<_>>();

        let mut handles = Vec::new();
        for command_buffer in command_buffers {
            handles.push(self.command_buffers.insert(command_buffer));
        }

        handles.try_into().unwrap_or_else(|_| {
            panic!(
                "Failed to convert command buffer handles into array of length {}",
                N
            )
        })
    }
}

pub mod util {
    use super::*;

    pub struct BlitImageInfo<'a> {
        pub src_image: &'a dyn Image,
        pub src_image_layout: vk::ImageLayout,
        pub dst_image: &'a dyn Image,
        pub dst_image_layout: vk::ImageLayout,
        pub src_subresource: vk::ImageSubresourceLayers,
        pub dst_subresource: vk::ImageSubresourceLayers,
        pub src_offset: vk::Offset3D,
        pub dst_offset: vk::Offset3D,
        pub src_extent: vk::Extent3D,
        pub dst_extent: vk::Extent3D,
        pub scaling: vk::Filter,
    }

    impl<'a> From<&BlitImageInfo<'a>> for vk::ImageBlit {
        fn from(info: &BlitImageInfo<'a>) -> Self {
            vk::ImageBlit::default()
                .src_subresource(info.src_subresource)
                .src_offsets([
                    info.src_offset,
                    vk::Offset3D {
                        x: info.src_offset.x + info.src_extent.width as i32,
                        y: info.src_offset.y + info.src_extent.height as i32,
                        z: info.src_offset.z + info.src_extent.depth as i32,
                    },
                ])
                .dst_subresource(info.dst_subresource)
                .dst_offsets([
                    info.dst_offset,
                    vk::Offset3D {
                        x: info.dst_offset.x + info.dst_extent.width as i32,
                        y: info.dst_offset.y + info.dst_extent.height as i32,
                        z: info.dst_offset.z + info.dst_extent.depth as i32,
                    },
                ])
        }
    }

    impl BlitImageInfo<'_> {
        pub fn with_defaults<'a>(
            src_image: &'a dyn Image,
            src_image_layout: vk::ImageLayout,
            dst_image: &'a dyn Image,
            dst_image_layout: vk::ImageLayout,
        ) -> BlitImageInfo<'a> {
            BlitImageInfo {
                src_image,
                src_image_layout,
                dst_image,
                dst_image_layout,
                src_subresource: src_image.instance().info().default_subresource_layers(),
                dst_subresource: dst_image.instance().info().default_subresource_layers(),
                src_offset: vk::Offset3D::default(),
                dst_offset: vk::Offset3D::default(),
                src_extent: src_image.instance().info().extent().into(),
                dst_extent: dst_image.instance().info().extent().into(),
                scaling: vk::Filter::NEAREST,
            }
        }
    }
}
