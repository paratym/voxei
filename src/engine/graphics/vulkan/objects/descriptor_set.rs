use std::sync::Arc;

use ash::vk;
use slotmap::{new_key_type, SlotMap};
use voxei_macros::VulkanResource;

use crate::engine::graphics::vulkan::{
    util::WeakGenericResourceDep,
    vulkan::{Vulkan, VulkanDep},
};

use super::{buffer::Buffer, image::Image};
use crate::engine::graphics::vulkan::util::VulkanResourceDep;

pub type DescriptorSetLayoutDep = Arc<DescriptorSetLayoutInstance>;

#[derive(VulkanResource)]
pub struct DescriptorSetLayoutInstance {
    vulkan_dep: VulkanDep,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

impl DescriptorSetLayoutInstance {
    pub fn layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

impl Drop for DescriptorSetLayoutInstance {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_dep
                .device()
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

pub struct DescriptorSetLayout {
    instance: Arc<DescriptorSetLayoutInstance>,
}

impl DescriptorSetLayout {
    pub fn builder() -> DescriptorSetLayoutBuilder<'static> {
        DescriptorSetLayoutBuilder::new()
    }

    pub fn instance(&self) -> &DescriptorSetLayoutInstance {
        &self.instance
    }

    pub fn create_dep(&self) -> DescriptorSetLayoutDep {
        self.instance.clone()
    }
}

pub struct DescriptorSetLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
}

impl DescriptorSetLayoutBuilder<'_> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn add_binding(
        &mut self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        descriptor_count: u32,
        stage_flags: vk::ShaderStageFlags,
    ) -> &mut Self {
        self.bindings.push(
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_type(descriptor_type)
                .descriptor_count(descriptor_count)
                .stage_flags(stage_flags),
        );
        self
    }

    pub fn build(self, vulkan: &Vulkan) -> DescriptorSetLayout {
        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::empty());

        // Safety: The descriptor set layout is dropped when the internal descriptor set layout is dropped
        let descriptor_set_layout = unsafe {
            vulkan
                .device()
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .expect("Failed to create descriptor set layout")
        };

        DescriptorSetLayout {
            instance: Arc::new(DescriptorSetLayoutInstance {
                vulkan_dep: vulkan.create_dep(),
                descriptor_set_layout,
            }),
        }
    }
}

new_key_type! { pub struct DescriptorSetHandle; }

pub struct DescriptorSet {
    descriptor_set: vk::DescriptorSet,
    written_dependencies: Vec<WeakGenericResourceDep>,
}

impl DescriptorSet {
    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.descriptor_set
    }

    pub fn written_dependencies(&self) -> &[WeakGenericResourceDep] {
        &self.written_dependencies
    }

    pub fn writer(&mut self, vulkan: &Vulkan) -> DescriptorSetWriter {
        DescriptorSetWriter::new(self, vulkan)
    }
}

pub struct DescriptorSetWriter<'a> {
    descriptor_set: &'a mut DescriptorSet,
    vulkan_dep: VulkanDep,
    writes: Vec<DescriptorWrite<'a>>,
    written_dependencies: Vec<WeakGenericResourceDep>,
}

struct DescriptorWrite<'a> {
    binding: u32,
    vk_type: vk::DescriptorType,
    write_type: DescriptorWriteType<'a>,
}

enum DescriptorWriteType<'a> {
    Buffer {
        buffer: &'a Buffer,
    },
    Image {
        image: &'a dyn Image,
        layout: vk::ImageLayout,
    },
}

impl<'a> DescriptorSetWriter<'a> {
    pub fn new(descriptor_set: &'a mut DescriptorSet, vulkan: &Vulkan) -> Self {
        Self {
            descriptor_set,
            vulkan_dep: vulkan.create_dep(),
            writes: Vec::new(),
            written_dependencies: Vec::new(),
        }
    }

    pub fn write_uniform_buffer(mut self, binding: u32, buffer: &'a Buffer) -> Self {
        self.written_dependencies
            .push(buffer.create_dep().into_generic_weak());

        self.writes.push(DescriptorWrite {
            binding,
            vk_type: vk::DescriptorType::UNIFORM_BUFFER,
            write_type: DescriptorWriteType::Buffer { buffer },
        });

        self
    }

    pub fn write_storage_image(
        mut self,
        binding: u32,
        image: &'a dyn Image,
        layout: vk::ImageLayout,
    ) -> Self {
        self.written_dependencies
            .push(Arc::downgrade(&image.create_generic_dep()));
        self.writes.push(DescriptorWrite {
            binding,
            vk_type: vk::DescriptorType::STORAGE_IMAGE,
            write_type: DescriptorWriteType::Image { image, layout },
        });

        self
    }

    pub fn write_storage_buffer(mut self, binding: u32, buffer: &'a Buffer) -> Self {
        self.written_dependencies
            .push(buffer.create_dep().into_generic_weak());

        self.writes.push(DescriptorWrite {
            binding,
            vk_type: vk::DescriptorType::STORAGE_BUFFER,
            write_type: DescriptorWriteType::Buffer { buffer },
        });

        self
    }

    pub fn submit_writes(self) {
        self.descriptor_set.written_dependencies = self.written_dependencies;

        let image_infos = self
            .writes
            .iter()
            .filter_map(|write| match &write.write_type {
                DescriptorWriteType::Buffer { .. } => None,
                DescriptorWriteType::Image { image, layout } => {
                    Some([vk::DescriptorImageInfo::default()
                        .image_layout(*layout)
                        .image_view(image.instance().image_view().unwrap())])
                }
            })
            .collect::<Vec<_>>();
        let buffer_infos = self
            .writes
            .iter()
            .filter_map(|write| match &write.write_type {
                DescriptorWriteType::Buffer { buffer } => {
                    Some([vk::DescriptorBufferInfo::default()
                        .buffer(buffer.instance().buffer())
                        .offset(0)
                        .range(vk::WHOLE_SIZE)])
                }
                DescriptorWriteType::Image { .. } => None,
            })
            .collect::<Vec<_>>();

        // this can so easily break lol
        let mut buffer_count = 0 as usize;
        let mut image_count = 0 as usize;
        let vk_writes = self
            .writes
            .iter()
            .map(|write| {
                let mut info = vk::WriteDescriptorSet::default()
                    .dst_set(self.descriptor_set.descriptor_set)
                    .dst_binding(write.binding)
                    .descriptor_type(write.vk_type);

                match write.write_type {
                    DescriptorWriteType::Buffer { .. } => {
                        info = info.buffer_info(&buffer_infos[buffer_count]);
                        buffer_count += 1;
                    }
                    DescriptorWriteType::Image { .. } => {
                        info = info.image_info(&image_infos[image_count]);
                        image_count += 1;
                    }
                }

                info
            })
            .collect::<Vec<_>>();

        unsafe {
            self.vulkan_dep
                .device()
                .update_descriptor_sets(&vk_writes, &[]);
        }
    }
}

pub struct DescriptorSetWriteStorageImageInfo<'a> {
    pub image: &'a dyn Image,
}

pub type DescriptorSetPoolDep = Arc<DescriptorSetPoolInstance>;

#[derive(VulkanResource)]
pub struct DescriptorSetPoolInstance {
    vulkan_dep: VulkanDep,
    descriptor_pool: vk::DescriptorPool,
}

impl Drop for DescriptorSetPoolInstance {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_dep
                .device()
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

pub struct DescriptorSetPool {
    instance: Arc<DescriptorSetPoolInstance>,
    descriptor_sets: SlotMap<DescriptorSetHandle, DescriptorSet>,
}

impl DescriptorSetPool {
    pub fn new(vulkan: &Vulkan) -> Self {
        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(100),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(100),
        ];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .max_sets(100);

        // Safety: The descriptor pool is dropped when the internal descriptor pool is dropped
        let descriptor_pool = unsafe {
            vulkan
                .device()
                .create_descriptor_pool(&descriptor_pool_create_info, None)
                .expect("Failed to create descriptor pool")
        };

        Self {
            instance: Arc::new(DescriptorSetPoolInstance {
                vulkan_dep: vulkan.create_dep(),
                descriptor_pool,
            }),
            descriptor_sets: SlotMap::with_key(),
        }
    }

    pub fn get(&self, handle: DescriptorSetHandle) -> Option<&DescriptorSet> {
        self.descriptor_sets.get(handle)
    }

    pub fn get_multiple(&self, handles: Vec<DescriptorSetHandle>) -> Vec<&DescriptorSet> {
        self.descriptor_sets
            .iter()
            .filter(|(handle, _)| handles.contains(handle))
            .map(|(_, descriptor_set)| descriptor_set)
            .collect()
    }

    pub fn get_mut(&mut self, handle: DescriptorSetHandle) -> Option<&mut DescriptorSet> {
        self.descriptor_sets.get_mut(handle)
    }

    pub fn get_multiple_mut(
        &mut self,
        handles: Vec<DescriptorSetHandle>,
    ) -> Vec<&mut DescriptorSet> {
        self.descriptor_sets
            .iter_mut()
            .filter(|(handle, _)| handles.contains(handle))
            .map(|(_, descriptor_set)| descriptor_set)
            .collect()
    }

    pub fn allocate_descriptor_sets<const N: usize>(
        &mut self,
        layout: &DescriptorSetLayout,
    ) -> [DescriptorSetHandle; N] {
        let descriptor_set_layouts = [layout.instance().layout(); N];

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.instance.descriptor_pool)
            .set_layouts(&descriptor_set_layouts);

        let descriptor_sets = unsafe {
            self.instance
                .vulkan_dep
                .device()
                .allocate_descriptor_sets(&descriptor_set_allocate_info)
                .expect("Failed to allocate descriptor sets")
        }
        .into_iter()
        .map(|descriptor_set| DescriptorSet {
            descriptor_set,
            written_dependencies: Vec::new(),
        })
        .collect::<Vec<_>>();

        let mut handles = [DescriptorSetHandle::default(); N];
        for (i, descriptor_set) in descriptor_sets.into_iter().enumerate() {
            handles[i] = self.descriptor_sets.insert(descriptor_set);
        }

        handles
    }

    pub fn create_dep(&self) -> DescriptorSetPoolDep {
        self.instance.clone()
    }
}
