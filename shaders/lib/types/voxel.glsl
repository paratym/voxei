struct VoxelNode {
  uint data_index;
  uint child_index;
  bool child_offsets[8];
};

struct VoxelData {
  vec3 normal;
};
