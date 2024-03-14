layout (local_size_x = 16, local_size_y = 1, local_size_z = 1) in;
#include "lib/types.glsl"
#include "lib/constants.glsl"
#include "lib/intersect.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId vertices_buffer; 
  ResourceId indices_buffer; 
  ResourceId voxel_buffer;
  uint32_t side_length;
  uint32_t tri_count;
} push_constants;

DECL_BUFFER(4) IndicesList {
  uint data[];
};

DECL_BUFFER(4) VerticesList {
  float data[];
};

DECL_BUFFER(4) VoxelList {
  int data[];
};

i32vec3 morton_to_world_position(uint32_t index, int half_length) {
  vec3 position = vec3(0);
  int curr_length = 1;
  while(curr_length <= half_length) {
    vec3 sign = vec3(
      ((index & 1) > 0) ? 1.0 : -1.0,
      ((index & 2) > 0) ? 1.0 : -1.0,
      ((index & 4) > 0) ? 1.0 : -1.0 
    );
    position += sign * curr_length * 0.5;
    index >>= 3;
    curr_length <<= 1;
  }
  return i32vec3(floor(position));
}

void main() {
  VerticesList vertices_list = get_buffer(push_constants.vertices_buffer, VerticesList);
  IndicesList indices_list = get_buffer(push_constants.indices_buffer, IndicesList);
  VoxelList voxel_list = get_buffer(push_constants.voxel_buffer, VoxelList);
  uint32_t side_length = push_constants.side_length;
  uint32_t half_length = side_length / 2;
  const uint32_t voxel_array_size = side_length * side_length * side_length; 

  uint32_t index = gl_GlobalInvocationID.x;

  if(index >= voxel_array_size) {
    return;
  }

  i32vec3 position = morton_to_world_position(index, int(half_length)); 
  vec3 voxel_min = vec3(position.x, position.y, position.z) * VOXEL_UNIT_LENGTH;
  vec3 voxel_max = voxel_min + VOXEL_UNIT_LENGTH;

  voxel_list.data[index] = 0;
  for(int i = 0; i < push_constants.tri_count; i++) {
    Triangle tri = Triangle(Vertex(vec3(
      vertices_list.data[indices_list.data[i * 3] * 3],
      vertices_list.data[indices_list.data[i * 3] * 3 + 1],
      vertices_list.data[indices_list.data[i * 3] * 3 + 2]
    )), Vertex(vec3(
      vertices_list.data[indices_list.data[i * 3 + 1] * 3],
      vertices_list.data[indices_list.data[i * 3 + 1] * 3 + 1],
      vertices_list.data[indices_list.data[i * 3 + 1] * 3 + 2]
    )), Vertex(vec3(
      vertices_list.data[indices_list.data[i * 3 + 2] * 3],
      vertices_list.data[indices_list.data[i * 3 + 2] * 3 + 1],
      vertices_list.data[indices_list.data[i * 3 + 2] * 3 + 2]
    )));
    vec3 tri_min = min(tri.v0.position, min(tri.v1.position, tri.v2.position));
    vec3 tri_max = max(tri.v0.position, max(tri.v1.position, tri.v2.position)); // So there is some difference to min max of tri if flat
    AABB tri_aabb = AABB(tri_min, tri_max);

    AABB voxel_aabb = AABB(voxel_min, voxel_max);

    // if(voxel_min.x != tri_min.x) {
    //   voxel_aabb.min.x += 0.0001;
    // }
    // if(voxel_max.x != tri_max.x) {
    //   voxel_aabb.max.x -= 0.0001;
    // }
    // if(voxel_min.y != tri_min.y) {
    //   voxel_aabb.min.y += 0.0001;
    // }
    // if(voxel_max.y != tri_max.y) {
    //   voxel_aabb.max.y -= 0.0001;
    // }
    // if(voxel_min.z != tri_min.z) {
    //   voxel_aabb.min.z += 0.0001;
    // }
    // if(voxel_max.z != tri_max.z) {
    //   voxel_aabb.max.z -= 0.0001;
    // }

    bool intersects = triangle_aabb_intersection(tri, voxel_aabb);
    if(intersects) {
      voxel_list.data[index] = intersects ? 1 : 0;
      break;
    }
  }
}
