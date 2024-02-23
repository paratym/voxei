use std::{fmt::Formatter, sync::Arc};

use ash::vk;
use voxei_macros::VulkanResource;

use crate::engine::graphics::vulkan::{
    allocator::{MemoryAllocation, VulkanAllocationInfo, VulkanMemoryAllocator},
    vulkan::{Vulkan, VulkanDep},
};

pub struct BufferInfo {
    size: u64,
}

impl BufferInfo {
    pub fn size(&self) -> u64 {
        self.size
    }
}

#[derive(VulkanResource)]
pub struct BufferInstance {
    vulkan_dep: VulkanDep,
    buffer: vk::Buffer,
    allocation: MemoryAllocation,
    info: BufferInfo,
}

impl BufferInstance {
    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }

    pub fn allocation(&self) -> &MemoryAllocation {
        &self.allocation
    }

    pub fn info(&self) -> &BufferInfo {
        &self.info
    }
}

impl Drop for BufferInstance {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_dep.device().destroy_buffer(self.buffer, None);
        }
    }
}

pub struct BufferCreateInfo {
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
    pub memory_usage: vk::MemoryPropertyFlags,
}

pub type BufferDep = Arc<BufferInstance>;

pub struct Buffer {
    instance: Arc<BufferInstance>,
}

impl Buffer {
    pub fn new(
        vulkan: &Vulkan,
        vulkan_memory_allocator: &mut VulkanMemoryAllocator,
        buffer_create_info: &BufferCreateInfo,
    ) -> Self {
        let buffer = unsafe {
            vulkan
                .device()
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(buffer_create_info.size)
                        .usage(buffer_create_info.usage),
                    None,
                )
                .unwrap()
        };

        let memory_requirements = unsafe { vulkan.device().get_buffer_memory_requirements(buffer) };

        let allocation_info = VulkanAllocationInfo {
            size: memory_requirements.size,
            memory_proprties: buffer_create_info.memory_usage,
            memory_type_bits: memory_requirements.memory_type_bits,
        };

        let allocation = vulkan_memory_allocator.allocate(&allocation_info);

        unsafe {
            vulkan
                .device()
                .bind_buffer_memory(buffer, allocation.instance().device_memory(), 0)
                .unwrap()
        };

        Self {
            instance: Arc::new(BufferInstance {
                vulkan_dep: vulkan.create_dep(),
                buffer,
                allocation,
                info: BufferInfo {
                    size: buffer_create_info.size,
                },
            }),
        }
    }

    pub fn instance(&self) -> &BufferInstance {
        &self.instance
    }

    pub fn create_dep(&self) -> BufferDep {
        self.instance.clone()
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("idc")
    }
}
