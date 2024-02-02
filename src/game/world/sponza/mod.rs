use std::{mem::size_of, ops::Deref};

use ash::vk;
use nalgebra::Vector3;
use voxei_macros::Resource;

use crate::engine::{
    assets::asset::{Assets, Handle},
    geometry::shapes::aabb::AABB,
    graphics::vulkan::{
        allocator::VulkanMemoryAllocator,
        objects::{
            buffer::{Buffer, BufferCreateInfo},
            glsl::{GlslDataBuilder, GlslFloat, GlslUInt, GlslVec3f},
        },
        vulkan::Vulkan,
    },
    model::mesh::Mesh,
    resource::{Res, ResMut},
    voxel::{octree::VoxelSVO, voxelizer},
};

pub const SPONZA_ASSET_PATH: &str = "assets/bunny.obj";
pub const SUBDIVISIONS: u32 = 4;

#[derive(Resource)]
pub struct Sponza {
    handle: Option<Handle<Vec<tobj::Model>>>,
    voxelized_octree: Option<VoxelSVO>,

    info: Option<Buffer>,
    gpu_nodes: Option<Buffer>,
    gpu_materials: Option<Buffer>,
}

impl Sponza {
    pub fn new() -> Self {
        Sponza {
            handle: None,
            voxelized_octree: None,
            info: None,
            gpu_nodes: None,
            gpu_materials: None,
        }
    }

    pub fn update(
        mut sponza: ResMut<Sponza>,
        mut assets: ResMut<Assets>,
        vulkan: Res<Vulkan>,
        mut vulkan_memory_allocator: ResMut<VulkanMemoryAllocator>,
    ) {
        if sponza.handle.is_none() {
            sponza.handle = Some(assets.load(SPONZA_ASSET_PATH));
        }

        let Some(handle) = &sponza.handle else {
            return;
        };

        if sponza.voxelized_octree.is_none() && handle.is_loaded() {
            println!("Voxelizing Sponza with grid length {}", 1 << SUBDIVISIONS);
            let mesh = Mesh::from(handle.get().unwrap().deref());
            let voxel_svo = voxelizer::voxelize(&mesh, SUBDIVISIONS);
            println!("Voxelized Sponza");

            let bbox = AABB::new_min_max(Vector3::new(0.0, 0.0, 0.0), Vector3::new(5.0, 5.0, 5.0));
            if bbox.max().x - bbox.min().x != bbox.max().y - bbox.min().y
                || bbox.max().x - bbox.min().x != bbox.max().z - bbox.min().z
            {
                panic!("Bbox is not a cube");
            }

            let unit_length = (bbox.max().x - bbox.min().x) / (1 << SUBDIVISIONS) as f32;

            // Save the voxelized octree to a file
            sponza.voxelized_octree = Some(voxel_svo);
            std::fs::write(
                "assets/sponza.svo",
                ron::ser::to_string_pretty(
                    &sponza.voxelized_octree.as_ref().unwrap(),
                    Default::default(),
                )
                .unwrap(),
            )
            .unwrap();

            // Do gpu stuff
            let node_count = sponza.voxelized_octree.as_ref().unwrap().nodes().len();
            let size = size_of::<u64>() + (size_of::<u64>() * 2 + (8)) * node_count;
            let gpu_nodes = Buffer::new(
                &vulkan,
                &mut vulkan_memory_allocator,
                &BufferCreateInfo {
                    size: size as u64,
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    memory_usage: vk::MemoryPropertyFlags::HOST_VISIBLE
                        | vk::MemoryPropertyFlags::HOST_COHERENT,
                },
            );

            let material_count = sponza.voxelized_octree.as_ref().unwrap().materials().len();
            let size = size_of::<u64>() + (size_of::<f32>() * 4) * material_count;
            println!("Material count: {}", material_count);
            let gpu_materials = Buffer::new(
                &vulkan,
                &mut vulkan_memory_allocator,
                &BufferCreateInfo {
                    size: size as u64,
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    memory_usage: vk::MemoryPropertyFlags::HOST_VISIBLE
                        | vk::MemoryPropertyFlags::HOST_COHERENT,
                },
            );
            let info = Buffer::new(
                &vulkan,
                &mut vulkan_memory_allocator,
                &BufferCreateInfo {
                    size: 4 * 6 + 8,
                    usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                    memory_usage: vk::MemoryPropertyFlags::HOST_VISIBLE
                        | vk::MemoryPropertyFlags::HOST_COHERENT,
                },
            );

            let info_ptr = info.instance().allocation().instance().map_memory(0) as *mut u8;

            let mut writer = GlslDataBuilder::new();
            writer.push(GlslVec3f::new(bbox.min().x, bbox.min().y, bbox.min().z));
            writer.push(GlslVec3f::new(bbox.max().x, bbox.max().y, bbox.max().z));
            writer.push(GlslFloat::new(unit_length));

            let data = writer.build();

            unsafe { info_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len()) };

            info.instance().allocation().instance().unmap_memory();

            let mut node_ptr =
                gpu_nodes.instance().allocation().instance().map_memory(0) as *mut u32;
            println!("Node count: {}", node_count);
            unsafe { node_ptr.write(node_count as u32) };
            node_ptr = unsafe { node_ptr.add(1) };

            for node in sponza.voxelized_octree.as_ref().unwrap().nodes() {
                unsafe {
                    node_ptr.write(node.data_index as u32);
                    node_ptr = node_ptr.add(1);
                    node_ptr.write(node.children_base_index as u32);
                    node_ptr = node_ptr.add(1);

                    let children_offset_ptr = node_ptr as *mut u8;
                    for i in 0..8 {
                        children_offset_ptr.add(i).write(node.children_offset[i]);
                    }
                    node_ptr = children_offset_ptr.add(8) as *mut u32;
                }
            }

            gpu_nodes.instance().allocation().instance().unmap_memory();

            let mut mat_ptr = gpu_materials
                .instance()
                .allocation()
                .instance()
                .map_memory(0) as *mut u32;
            unsafe { mat_ptr.write(material_count as u32) };
            mat_ptr = unsafe { mat_ptr.add(1) };
            let mut mat_ptr = mat_ptr as *mut f32;

            for mat in sponza.voxelized_octree.as_ref().unwrap().materials() {
                unsafe {
                    mat_ptr.write(mat.normal[0]);
                    mat_ptr = mat_ptr.add(1);
                    mat_ptr.write(mat.normal[1]);
                    mat_ptr = mat_ptr.add(1);
                    mat_ptr.write(mat.normal[2]);
                    mat_ptr = mat_ptr.add(2);
                }
            }

            gpu_materials
                .instance()
                .allocation()
                .instance()
                .unmap_memory();

            sponza.gpu_nodes = Some(gpu_nodes);
            sponza.gpu_materials = Some(gpu_materials);
            sponza.info = Some(info);
        }
    }

    pub fn gpu_nodes(&self) -> Option<&Buffer> {
        self.gpu_nodes.as_ref()
    }

    pub fn gpu_materials(&self) -> Option<&Buffer> {
        self.gpu_materials.as_ref()
    }

    pub fn info(&self) -> Option<&Buffer> {
        self.info.as_ref()
    }

    pub fn is_ready(&self) -> bool {
        self.gpu_nodes.is_some() && self.gpu_materials.is_some() && self.info.is_some()
    }
}
