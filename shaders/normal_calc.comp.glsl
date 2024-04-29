layout (local_size_x = 512, local_size_y = 1, local_size_z = 1) in;

#include "lib/constants.glsl"
#include "lib/types.glsl"
#include "lib/utils.glsl"

DECL_BUFFER(4) BrickProcessList {
  uint32_t bricks[];
};

DECL_PUSH_CONSTANTS {
  ResourceId voxel_world_info_id;
  ResourceId to_process_bricks;
} push_constants;

const int CHECK_LENGTH = 1;
const float CHECK_LENGTH_DIST = length(vec3(CHECK_LENGTH, CHECK_LENGTH, CHECK_LENGTH));

bool check_voxel_occupancy(i32vec3 i_voxel_position) {
  VoxelWorldInfo info = get_buffer(push_constants.voxel_world_info_id, VoxelWorldInfo);
  BrickIndicesGrid brick_indices_grid = get_buffer(info.brick_indices_grid_buffer, BrickIndicesGrid);
  BrickDataList brick_data_list = get_buffer(info.brick_data_buffer, BrickDataList);

  if (i_voxel_position.x < 0 || i_voxel_position.y < 0 || i_voxel_position.z < 0) {
    return true;
  }
  uint32_t maximum_voxel_pos = info.chunk_side_length * CHUNK_LENGTH * BRICK_LENGTH;
  if (i_voxel_position.x >= maximum_voxel_pos || i_voxel_position.y >= maximum_voxel_pos || i_voxel_position.z >= maximum_voxel_pos) {
    return true;
  }
  u32vec3 voxel_position = u32vec3(i_voxel_position + i32vec3(0,0,0));

  u32vec3 brick_position = u32vec3(floor(vec3(voxel_position) / vec3(BRICK_LENGTH)));
  u32vec3 voxel_local = voxel_position % BRICK_LENGTH;
  uint32_t brick_morton = morton_encode_3(brick_position.x, brick_position.y, brick_position.z);
  uint32_t voxel_morton = morton_encode_3(voxel_local.x, voxel_local.y, voxel_local.z);

  u32vec3 d_vox_pos = voxel_position - u32vec3(0, 1, 0);
  u32vec3 d_b_pos = d_vox_pos / BRICK_LENGTH;
  uint32_t d_morton = morton_encode_3(d_b_pos.x, d_b_pos.y, d_b_pos.z);

  uint32_t brick_index = brick_indices_grid.grid[brick_morton];
  uint32_t brick_status = brick_index >> 30;
  if(brick_position.y == 129) {
    debugPrintfEXT("voxel y is %d\n", voxel_position.y);
    return true;
  }
  if(brick_status == 3) {
    // debugPrintfEXT("brick accessed %d %d %d\n", brick_position.x, brick_position.y, brick_position.z);
    // debugPrintfEXT("brick out of bounds %d\n", brick_morton);
    return false;
  } else {
    // debugPrintfEXT("brick not weewoo %d %d %d\n", brick_position.x, brick_position.y, brick_position.z);
  }
  return false;
  if(brick_status == 0 || brick_status == 1) {
    debugPrintfEXT("voxel out of bounds %d %d %d\n", i_voxel_position.x, i_voxel_position.y, i_voxel_position.z);
    return true;
  }
  return false;
  uint32_t brick_data_index = brick_index & 0x3FFFFFFF;
  BrickData brick_data = brick_data_list.data[brick_index];
  uint32_t voxel_status = brick_data.voxel_mask[voxel_morton >> 3] & (1 << (voxel_morton & 7));
  if (voxel_status != 0) {
    // debugPrintfEXT("check brick inde xdata %d\n", brick_data_index);
    //debugPrintfEXT("\n\n\nbrick local position %d %d %d\n", brick_position.x, brick_position.y, brick_position.z);
    return true;
  }
  return false;
}

void main() {
  VoxelWorldInfo info = get_buffer(push_constants.voxel_world_info_id, VoxelWorldInfo);
  BrickIndicesGrid brick_indices = get_buffer(info.brick_indices_grid_buffer, BrickIndicesGrid);
  BrickDataList brick_data_list = get_buffer(info.brick_data_buffer, BrickDataList);
  BrickPaletteIndicesList brick_palette_indices_list = get_buffer(info.brick_palette_indices_list_buffer, BrickPaletteIndicesList);
  BrickPaletteListVolatile palette_list = get_buffer(info.brick_palette_list_buffer, BrickPaletteListVolatile);

  BrickProcessList to_process = get_buffer(push_constants.to_process_bricks, BrickProcessList);
  uint32_t to_process_index = gl_GlobalInvocationID.x / 512;
  uint32_t current_brick_morton = to_process.bricks[to_process_index];
  uint32_t brick_data_index = brick_indices.grid[current_brick_morton] & 0x3FFFFFFF;
  BrickData current_brick_data = brick_data_list.data[brick_data_index];
  uint32_t current_voxel_morton = gl_GlobalInvocationID.x % 512;
  uint32_t voxel_status = current_brick_data.voxel_mask[current_voxel_morton >> 3] & (1 << (current_voxel_morton & 7));
  // dont process empty voxels
  // debugPrintfEXT("current brick index %d\n", current_brick_morton);
  uint32_t palette_index = current_brick_data.palette_index & 0x3FFFFFFF;
  u32vec3 voxel_local_position = morton_decode_3(current_voxel_morton);
  u32vec3 brick_local_position = morton_decode_3(current_brick_morton);
  // debugPrintfEXT("processing brick %d, %d, %d\n", brick_local_position.x, brick_local_position.y, brick_local_position.z);
  u32vec3 voxel_world_position = brick_local_position * BRICK_LENGTH + voxel_local_position;
  if(voxel_status == 0) {
    return;
  }
  if ((gl_GlobalInvocationID.x % 512) == 0) {
    //debugPrintfEXT("processing brick %d, %d, %d\n", brick_local_position.x, brick_local_position.y, brick_local_position.z);
  }


  // calculate voxel norml
  vec3 normal = vec3(0,0,1);
  vec3 alb = vec3(0,0,0);
  int i = 0;
  // for(int x = -CHECK_LENGTH; x <= CHECK_LENGTH; x++) {
  //   for(int y = -CHECK_LENGTH; y <= CHECK_LENGTH; y++) {
  //     for(int z = -CHECK_LENGTH; z <= CHECK_LENGTH; z++) {
  //       // skip current voxel.
  //       if (x == 0 && y == 0 && z == 0) {
  //         continue;
  //       }
  //       // if(!(x == 0 && y == 1 && z == 0)) {
  //       //   continue;
  //       // }

  //       i32vec3 neighbour_voxel_position = i32vec3(voxel_world_position) + i32vec3(x, y, z);
  //       if(!check_voxel_occupancy(neighbour_voxel_position, info, brick_indices, brick_data_list)) {
  //         float check_length = length(vec3(x, y, z));
  //         normal += vec3(x, y, z) / check_length;
  //       } else {
  //         i++;
  //       }
  //     }
  //   }
  // }
  if(current_brick_morton == 14811753) {
    normal = vec3(1, 1, 1);
  }
  if(current_brick_morton == 14811757) {
    if(!check_voxel_occupancy(i32vec3(voxel_world_position) + i32vec3(0,1,0))) {
      // normal = vec3(1, 0, 0);
    }
  } else {
    //normal = vec3(1, 1, 0);
    // debugPrintfEXT("voxel world position %d %d %d\n", voxel_world_position.x, voxel_world_position.y, voxel_world_position.z);
  }
  if(voxel_local_position.y == 7) {
    normal = vec3(1,0,0);
  }
  alb = vec3(voxel_local_position) / 7.0;

  // Write voxel_normal 
  uint32_t octa_norm = octahedral_32(normalize(normal), 4);
  uint32_t albedo = (uint32_t(alb.x * 255) << 16) | (uint32_t(alb.y * 255) << 8)| uint32_t(alb.z * 255);
  uint32_t voxel_index = brick_palette_indices_list.indices[brick_data_index * BRICK_VOLUME + current_voxel_morton];
  uint32_t pv = palette_list.voxels[palette_index + voxel_index];
  palette_list.voxels[palette_index + voxel_index] = (octa_norm << 24) | (albedo & 0xFFFFFF);
}
