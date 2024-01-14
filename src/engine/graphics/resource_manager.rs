use std::collections::HashMap;

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use voxei_macros::Resource;

use crate::constants;

use super::vulkan::{
    allocator::VulkanMemoryAllocator,
    objects::{
        descriptor_set::{
            DescriptorSet, DescriptorSetHandle, DescriptorSetLayout, DescriptorSetPool,
        },
        image::{OwnedImage, OwnedImageCreateInfo},
    },
    vulkan::Vulkan,
};

#[derive(Resource)]
pub struct RenderResourceManager {
    images: HashMap<String, OwnedImage>,
    descriptor_sets: HashMap<String, [DescriptorSetHandle; constants::FRAMES_IN_FLIGHT]>,
    descriptor_pool: RwLock<DescriptorSetPool>,
}

impl RenderResourceManager {
    pub fn new(vulkan: &Vulkan) -> Self {
        let descriptor_pool = DescriptorSetPool::new(vulkan);

        Self {
            images: HashMap::new(),
            descriptor_sets: HashMap::new(),
            descriptor_pool: RwLock::new(descriptor_pool),
        }
    }

    /// Gets a stored image with the specified name or creates a new one if it doesn't exist with
    /// the supplied info.
    pub fn create_image(
        &mut self,
        vulkan: &Vulkan,
        vulkan_memory_allocator: &mut VulkanMemoryAllocator,
        name: &str,
        info: &OwnedImageCreateInfo,
    ) {
        self.images.insert(
            name.to_string(),
            OwnedImage::new(vulkan, vulkan_memory_allocator, info),
        );
    }

    pub fn has_image(&self, name: &str) -> bool {
        self.images.contains_key(name)
    }

    pub fn get_image(&self, name: &str) -> Option<&OwnedImage> {
        self.images.get(name)
    }

    pub fn create_descriptor_sets(&mut self, name: &str, layout: &DescriptorSetLayout) {
        self.descriptor_sets.insert(
            name.to_string(),
            self.descriptor_pool
                .write()
                .allocate_descriptor_sets(layout),
        );
    }

    pub fn has_descriptor_sets(&self, name: &str) -> bool {
        self.descriptor_sets.contains_key(name)
    }

    pub fn get_descriptor_set(
        &self,
        name: &str,
        frame_index: usize,
    ) -> Option<MappedRwLockReadGuard<DescriptorSet>> {
        self.descriptor_sets.get(name).map(|handles| {
            RwLockReadGuard::map(self.descriptor_pool.read(), |pool| {
                pool.get(handles[frame_index]).unwrap()
            })
        })
    }

    pub fn get_descriptor_set_handle(
        &self,
        name: &str,
        frame_index: usize,
    ) -> Option<DescriptorSetHandle> {
        self.descriptor_sets
            .get(name)
            .map(|handles| handles[frame_index])
    }

    pub fn get_descriptor_set_mut(
        &self,
        name: &str,
        frame_index: usize,
    ) -> Option<MappedRwLockWriteGuard<DescriptorSet>> {
        self.descriptor_sets.get(name).map(|handles| {
            RwLockWriteGuard::map(self.descriptor_pool.write(), |pool| {
                pool.get_mut(handles[frame_index]).unwrap()
            })
        })
    }

    pub fn get_descriptor_pool(&self) -> MappedRwLockReadGuard<DescriptorSetPool> {
        RwLockReadGuard::map(self.descriptor_pool.read(), |pool| pool)
    }
}
