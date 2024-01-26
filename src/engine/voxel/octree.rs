pub type MortonCode = u32;

#[derive(Clone)]
pub struct VoxelData {
    pub morton_code: MortonCode,
    pub normal: [f32; 3],
}

impl VoxelData {
    pub fn empty() -> Self {
        Self {
            morton_code: 0,
            normal: [0.0; 3],
        }
    }
}

pub struct VoxelMaterial {
    pub normal: [f32; 3],
}

#[derive(Clone)]
pub struct SVONode {
    pub data_index: usize,
    pub children_base_index: usize,
    pub children_offset: [u8; 8],
}

impl SVONode {
    pub fn empty() -> Self {
        Self {
            data_index: 0,
            children_base_index: 0,
            children_offset: [u8::MAX; 8],
        }
    }

    pub fn has_data(&self) -> bool {
        self.data_index == 0
    }

    pub fn is_leaf(&self) -> bool {
        self.children_base_index == 0
    }

    pub fn is_null(&self) -> bool {
        !self.has_data() && self.is_leaf()
    }
}

pub struct VoxelSVO {
    nodes: Vec<SVONode>,
    material: Vec<VoxelMaterial>,
    unit_length: f32,
}

impl VoxelSVO {
    pub fn nodes(&self) -> &[SVONode] {
        &self.nodes
    }

    pub fn materials(&self) -> &[VoxelMaterial] {
        &self.material
    }

    pub fn unit_length(&self) -> f32 {
        self.unit_length
    }
}

pub struct VoxelSVOBuilder {
    pub current_morton_code: MortonCode,
    pub max_depth: u32,

    pub buffers: Vec<Vec<SVONode>>,
    pub svo_nodes: Vec<SVONode>,
    pub svo_data: Vec<VoxelData>,
}

impl VoxelSVOBuilder {
    pub fn new(grid_length: usize) -> Self {
        let max_depth = (grid_length as f32).log2().ceil() as u32;

        let buffers = vec![vec![SVONode::empty(); 8]; max_depth as usize + 1];

        Self {
            current_morton_code: 0,
            max_depth,
            buffers,
            svo_nodes: Vec::new(),
            svo_data: Vec::new(),
        }
    }

    pub fn add_voxel(&mut self, data: VoxelData) {
        // Fill in empty voxels
        if data.morton_code > self.current_morton_code {
            self.fill_empty_voxels((data.morton_code - self.current_morton_code) as usize);
        }

        // Add voxel data
        self.svo_data.push(data);

        // Add voxel node to max depth buffer
        let node = SVONode {
            data_index: self.svo_data.len() - 1,
            children_base_index: 0,
            children_offset: [0; 8],
        };
        self.buffers[(self.max_depth) as usize].push(node);

        // Refine buffers
        self.refine_buffers();
    }

    // Groups common voxels into higher level buffers, writes any remaining voxels to svo
    fn refine_buffers(&mut self) {
        for depth in (1..=self.max_depth).rev() {
            let depth = depth as usize;
            if self.buffers[depth as usize].len() == 8 {
                let is_buffer_empty = self.buffers[depth as usize]
                    .iter()
                    .all(|node| node.data_index == 0);

                if is_buffer_empty {
                    self.buffers[depth - 1].push(SVONode::empty());
                } else {
                    let node = self.group_buffer(depth);
                    self.buffers[depth - 1].push(node);
                }
            } else {
                break;
            }
        }
    }

    fn group_buffer(&mut self, depth: usize) -> SVONode {
        let mut parent = SVONode::empty();

        let mut is_first = true;
        for i in 0..8 {
            if self.buffers[depth][i].is_null() {
                continue;
            }

            let node = self.buffers[depth][i].clone();

            if is_first {
                parent.children_base_index = self.svo_nodes.len();
                is_first = false;
            }
            let offset = self.svo_nodes.len() - parent.children_base_index;
            parent.children_offset[i] = offset as u8;

            self.svo_nodes.push(node);
        }

        parent
    }

    pub fn finalize_svo(mut self, unit_length: f32) -> VoxelSVO {
        let final_mortan_code = 8u32.pow(self.max_depth);

        // Fill in empty voxels
        self.fill_empty_voxels((final_mortan_code - self.current_morton_code) as usize);

        // Add root node
        self.svo_nodes.push(self.buffers[0][0].clone());

        VoxelSVO {
            nodes: self.svo_nodes,
            material: Vec::new(),
            unit_length,
        }
    }

    fn fill_empty_voxels(&mut self, mut size: usize) {
        while size > 0 {
            self.add_empty_voxel(self.max_depth);
            size -= 1;
        }
    }

    fn add_empty_voxel(&mut self, depth: u32) {
        self.buffers[depth as usize].push(SVONode::empty());
        self.refine_buffers();
    }
}
