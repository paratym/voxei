struct Ray {
  vec3 origin;
  vec3 dir;
  vec3 inv_dir;
};

struct SVONode {
  uint data_index;
  uint child_index;
  uint child_offsets[2];
};

struct VoxelData {
  vec3 normal;
};

struct AABB {
  vec3 min;
  vec3 max;
};

struct Vertex {
  vec3 position;
};

struct Triangle {
  Vertex a;
  Vertex b;
  Vertex c;
};
