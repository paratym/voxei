use std::sync::Arc;

use ash::vk;
use voxei_macros::Resource;

use super::vulkan::{Vulkan, VulkanDep};

pub struct MemoryAllocation {
    instance: Arc<MemoryAllocationInstance>,
}

impl MemoryAllocation {
    pub fn instance(&self) -> &MemoryAllocationInstance {
        &self.instance
    }
}

pub struct MemoryAllocationInstance {
    vulkan_dep: VulkanDep,
    device_memory: vk::DeviceMemory,
    size: u64,
}

impl MemoryAllocationInstance {
    pub fn device_memory(&self) -> vk::DeviceMemory {
        self.device_memory
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn map_memory(&self, offset: u64) -> *mut std::ffi::c_void {
        unsafe {
            self.vulkan_dep
                .device()
                .map_memory(
                    self.device_memory,
                    offset,
                    self.size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map memory")
        }
    }

    pub fn unmap_memory(&self) {
        unsafe {
            self.vulkan_dep.device().unmap_memory(self.device_memory);
        }
    }
}

impl Drop for MemoryAllocationInstance {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_dep
                .device()
                .free_memory(self.device_memory, None);
        }
    }
}

#[derive(Resource)]
pub struct VulkanMemoryAllocator {
    vulkan_dep: VulkanDep,
}

pub struct VulkanAllocationInfo {
    pub size: u64,
    pub memory_proprties: vk::MemoryPropertyFlags,
    pub memory_type_bits: u32,
}

impl VulkanMemoryAllocator {
    pub fn new(vulkan: &Vulkan) -> Self {
        Self {
            vulkan_dep: vulkan.create_dep(),
        }
    }

    pub fn allocate(&mut self, info: &VulkanAllocationInfo) -> MemoryAllocation {
        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(info.size)
            .memory_type_index(
                self.find_memory_type_index(info.memory_type_bits, info.memory_proprties),
            );

        let device_memory = unsafe {
            self.vulkan_dep
                .device()
                .allocate_memory(&memory_allocate_info, None)
                .expect("Failed to allocate memory")
        };

        MemoryAllocation {
            instance: Arc::new(MemoryAllocationInstance {
                vulkan_dep: self.vulkan_dep.clone(),
                device_memory,
                size: info.size,
            }),
        }
    }

    fn find_memory_type_index(
        &self,
        memory_type_bits: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        self.vulkan_dep
            .physical_device()
            .memory_properties()
            .memory_types
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (memory_type_bits & (1 << index)) != 0
                    && memory_type.property_flags.contains(properties)
            })
            .map(|(index, _)| index as u32)
            .expect("Failed to find memory type index")
    }
}
