use voxei_macros::Resource;

use crate::engine::{
    assets::asset::{Assets, Handle},
    graphics::vulkan::objects::buffer::Buffer,
    resource::ResMut,
    voxel::{
        octree::VoxelSVO,
        voxelizer::{TriReader, Voxelizer},
    },
};

pub const SPONZA_ASSET_PATH: &str = "assets/sponza/sponza.obj";
pub const SUBDIVISIONS: u32 = 4;

#[derive(Resource)]
pub struct Sponza {
    handle: Option<Handle<Vec<tobj::Model>>>,
    voxelized_octree: Option<VoxelSVO>,

    gpu_nodes: Option<Buffer>,
    gpu_materials: Option<Buffer>,
}

impl Sponza {
    pub fn new() -> Self {
        Sponza {
            handle: None,
            voxelized_octree: None,
            gpu_nodes: None,
            gpu_materials: None,
        }
    }

    pub fn update(mut sponza: ResMut<Sponza>, mut assets: ResMut<Assets>) {
        if sponza.handle.is_none() {
            sponza.handle = Some(assets.load(SPONZA_ASSET_PATH));
        }

        let Some(handle) = &sponza.handle else {
            return;
        };

        if sponza.voxelized_octree.is_none() && handle.is_loaded() {
            println!("Sponza loaded");
            let reader = TriReader::new(&handle.get().unwrap());
            let grid_length = 1 << SUBDIVISIONS;

            println!("Voxelizing Sponza with grid length {}", grid_length);
            let voxelizer = Voxelizer::new(reader, grid_length);
            sponza.voxelized_octree = Some(voxelizer.voxelize());
            println!("Voxelized Sponza");

            // Copy to gpu
        }
    }
}
