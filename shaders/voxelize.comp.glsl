layout (local_size_x = 16, local_size_y = 1, local_size_z = 1) in;
#include "lib/types.glsl"
#include "lib/constants.glsl"
#include "lib/intersect.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId triangle_buffer; 
  ResourceId voxel_buffer;
  uint32_t side_length;
} push_constants;

DECL_BUFFER(16) TriangleList {
  uint32_t data_length;
  Triangle data[];
};

DECL_BUFFER(4) VoxelList {
  int data[];
};

i32vec3 morton_to_position(uint32_t index, int half_length) {
  i32vec3 position = i32vec3(0);
  int curr_length = 1;
  while(true) {
    i32vec3 sign = i32vec3(
      ((index & 1) > 0) ? curr_length - 1 : -curr_length,
      ((index & 4) > 0) ? curr_length - 1 : -curr_length,
      ((index & 2) > 0) ? curr_length - 1 : -curr_length
    );
    position += sign;

    if(curr_length == half_length) {
      return position;
    }

    index = index >> 3;
    curr_length <<= 1;
  }
  return position;
}

void main() {
  TriangleList triangle_list = get_buffer(push_constants.triangle_buffer, TriangleList);
  VoxelList voxel_list = get_buffer(push_constants.voxel_buffer, VoxelList);
  uint32_t side_length = push_constants.side_length;
  uint32_t half_length = side_length / 2;
  const uint32_t voxel_array_size = side_length * side_length * side_length; 

  uint32_t index = gl_GlobalInvocationID.x;

  if(index >= voxel_array_size) {
    return;
  }

  i32vec3 position = morton_to_position(index, int(half_length)); 
  vec3 min = vec3(position.x, position.y, position.z) * VOXEL_UNIT_LENGTH;
  vec3 max = min + VOXEL_UNIT_LENGTH;
  AABB voxel_world_bounds = AABB(min, max);

  voxel_list.data[index] = 0;
  for(int i = 0; i < triangle_list.data_length; i++) {
    Triangle triangle = triangle_list.data[i];
    // TODO: Actual intersection code using SAP
    // bool intersects = triangle_aabb_intersection(triangle, voxel_world_bounds);
    bool intersects = (index % 2) == 0;
    
    if(intersects) {
      voxel_list.data[index] = intersects ? 1 : 0;
      break;
    }
  }
}
