pub struct StaticVoxelWorld {}

impl StaticVoxelWorld {}

// Represents a 64 child tree.
struct InternalNode {
    child_base_index: u32,
    child_offset: u64,
}
