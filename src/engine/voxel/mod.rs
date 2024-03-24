use nalgebra::Vector3;

pub mod octree;
pub mod vox_world;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(C)]
pub struct VoxelData {
    color: (f32, f32, f32),
}

impl Eq for VoxelData {}

impl std::hash::Hash for VoxelData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.color.0.to_le_bytes().hash(state);
        self.color.1.to_le_bytes().hash(state);
        self.color.2.to_le_bytes().hash(state);
    }
}
