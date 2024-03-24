struct Ray {
  vec3 origin;
  vec3 dir;
  vec3 inv_dir;
};

struct AABB {
  vec3 min;
  vec3 max;
};

struct RayAABBIntersection {
  vec3 tmin;
  vec3 tmax;
  float tenter;
  float texit;
  bool hit;
};

struct Vertex {
  vec3 position;
};

struct Triangle {
  Vertex v0;
  Vertex v1;
  Vertex v2;
};

DECL_BUFFER(16) Camera {
  mat4 transform;
  mat4 view;
  mat4 projView;
  u32vec2 resolution;
  float aspect;
  float fov;
};

const uint32_t NULL_NODE = ~0;
const uint32_t NODE_TYPE_VOXEL = 0;
const uint32_t NODE_TYPE_CHUNK = 1;

struct VoxelOctreeNode {
  uint32_t voxel_data;
  uint32_t children[8];
};

struct ChunkOctreeNode {
  uint32_t chunk_data;
  uint32_t children[8];
};

struct VoxelData {
  vec3 color;
};

VoxelData voxel_data_empty() {
  return VoxelData(vec3(0, 0, 0));
}

DECL_BUFFER(16) VoxelOctree {
  VoxelOctreeNode nodes[];
};

DECL_BUFFER(4) ChunkOctree {
  uint32_t side_length;
  ChunkOctreeNode nodes[];
};

DECL_BUFFER(16) ChunkDataLUT {
  // Pointers to VoxelOctree buffers.
  ResourceId chunks[];
};

DECL_BUFFER(16) VoxelDataList {
  VoxelData data[];
};
