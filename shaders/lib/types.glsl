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

DECL_BUFFER(4) ChunkOccupancyGrid {
  uint16_t grid[];
};

DECL_BUFFER_COHERENT(4) BrickIndicesGrid {
  // First two most signifigant bits are status flag.
  uint32_t grid[];
};

struct BrickData {
  uint32_t material_index;
  uint8_t voxel_mask[BRICK_AREA];
};

DECL_BUFFER(4) BrickDataList {
  BrickData data[];
};

struct BrickMaterialData {
  uint64_t voxels[BRICK_VOLUME]; 
};

DECL_BUFFER(4) BrickMaterialDataList {
  BrickMaterialData data[];
};

struct BrickRequest {
  uint64_t morton;
};

DECL_BUFFER_COHERENT(4) BrickRequestList {
  uint32_t ptr;
  BrickRequest data[];
};

DECL_BUFFER(16) VoxelWorldInfo {
  i32vec3 chunk_center;
  u32vec3 chunk_translation;
  ResourceId chunk_occupancy_grid_buffer;
  ResourceId brick_indices_grid_buffer;
  ResourceId brick_data_buffer;
  ResourceId brick_material_buffer;
  ResourceId brick_request_list_buffer;

  uint32_t chunk_side_length;
  uint32_t chunk_half_length;
};

struct VoxelMaterial {
  vec3 albedo;
  vec3 normal;
};
