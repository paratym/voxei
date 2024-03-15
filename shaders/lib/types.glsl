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
