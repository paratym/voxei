/// When referrind to length, size, etc. of voxels existing in world-space, we want to use metric
/// and base things off of real world units, walk speed, etc. off of that. We will follow that
/// something such as CHUNK_REAL_SIZE is 16 meters in length.

pub const CHUNK_OCTREE_DEPTH: u8 = 4;
pub const CHUNK_OCTREE_SIZE: u32 = 1 << CHUNK_OCTREE_DEPTH;

pub const CHUNK_REAL_SIZE: f32 = 16.0;
pub const CHUNK_VOXEL_SIZE: f32 = CHUNK_REAL_SIZE / CHUNK_OCTREE_SIZE as f32;
