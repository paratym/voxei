use super::{
    vox_world::{CHUNK_LENGTH, CHUNK_OCTREE_HEIGHT},
    VoxelData,
};

pub const ROOT_INDEX: u32 = 0;
pub const NULL_INDEX: u32 = u32::MAX;

#[derive(Debug)]
#[repr(C)]
pub struct VoxelOctreeNode {
    // First bit signifies if the node is "non-occupied" in terms of the octree array.
    voxel_data: u32,
    children_pointers: [u32; 8],
}

impl VoxelOctreeNode {
    pub fn empty() -> Self {
        Self {
            voxel_data: NULL_INDEX,
            children_pointers: [NULL_INDEX; 8],
        }
    }
}

#[derive(Debug)]
pub struct VoxelOctree {
    nodes: Vec<VoxelOctreeNode>,
}

impl VoxelOctree {
    pub fn new() -> Self {
        VoxelOctree {
            nodes: vec![VoxelOctreeNode::empty()],
        }
    }

    pub fn add_voxel(&mut self, mut voxel_morton: u32, voxel_data: u32) {
        let mut node_index = ROOT_INDEX;

        for i in (0..CHUNK_OCTREE_HEIGHT).rev() {
            let curr_node = &self.nodes[node_index as usize];

            let child_morton = ((voxel_morton >> (3 * i)) & 0b111) as usize;
            voxel_morton <<= 3;

            let prev_node = node_index;
            node_index = curr_node.children_pointers[child_morton];
            if node_index == NULL_INDEX {
                let new_index = self.nodes.len() as u32;
                self.nodes.push(VoxelOctreeNode::empty());
                self.nodes[prev_node as usize].children_pointers[child_morton] = new_index;
                node_index = new_index;
            }
        }

        self.nodes[node_index as usize].voxel_data = voxel_data;
    }

    pub fn nodes(&self) -> &Vec<VoxelOctreeNode> {
        &self.nodes
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ChunkOctreeNode {
    chunk_data: u32,
    children_pointers: [u32; 8],
}

impl ChunkOctreeNode {
    pub fn empty() -> Self {
        Self {
            chunk_data: NULL_INDEX,
            children_pointers: [NULL_INDEX; 8],
        }
    }
}

#[derive(Debug)]
pub struct ChunkOctree {
    nodes: Vec<ChunkOctreeNode>,
    side_length: u32,
}

impl ChunkOctree {
    pub fn new(side_length: u32) -> Self {
        Self {
            nodes: vec![ChunkOctreeNode::empty()],
            side_length,
        }
    }

    pub fn create_chunk(&mut self, mut chunk_morton: u32, chunk_data: u32) {
        let mut node_index = ROOT_INDEX;

        for i in (0..self.height()).rev() {
            let curr_node = &self.nodes[node_index as usize];

            let child_morton = ((chunk_morton >> (i * 3)) & 0b111) as usize;
            chunk_morton <<= 3;

            let prev_node = node_index;
            node_index = curr_node.children_pointers[child_morton];
            if node_index == NULL_INDEX {
                let new_index = self.nodes.len() as u32;
                self.nodes.push(ChunkOctreeNode::empty());
                self.nodes[prev_node as usize].children_pointers[child_morton] = new_index;
                node_index = new_index;
            }
        }

        self.nodes[node_index as usize].chunk_data = chunk_data;
    }

    pub fn get_chunk(&self, mut chunk_morton: u32) -> u32 {
        let mut node_index = ROOT_INDEX;

        for _ in 0..self.height() {
            let curr_node = &self.nodes[node_index as usize];
            let child_morton = (chunk_morton & 0b111) as usize;
            chunk_morton >>= 3;

            node_index = curr_node.children_pointers[child_morton];
            if node_index == NULL_INDEX {
                return NULL_INDEX;
            }
        }

        self.nodes[node_index as usize].chunk_data
    }

    pub fn side_length(&self) -> u32 {
        self.side_length
    }

    pub fn height(&self) -> u32 {
        println!("height: {}", self.side_length.trailing_zeros());
        self.side_length.trailing_zeros()
    }

    pub fn nodes(&self) -> &Vec<ChunkOctreeNode> {
        &self.nodes
    }
}
