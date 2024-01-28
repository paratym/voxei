struct VoxelNode {
  uint data_index;
  uint child_index;
  uint child_offsets[2];
};

struct VoxelData {
  vec3 normal;
};


