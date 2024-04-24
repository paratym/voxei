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
  uint32_t current_voxel_morton = gl_LocalInvocationID.x;
  uint32_t voxel_status = current_brick_data.voxel_mask[current_voxel_morton >> 3] & (1 << (current_voxel_morton & 7));
  // dont process empty voxels
  if(voxel_status == 0) {
    return;
  }
  // debugPrintfEXT("current brick index %d\n", current_brick_morton);
  uint32_t palette_index = current_brick_data.palette_index & 0x3FFFFFFF;
  uint32_t voxel_index = brick_palette_indices_list.indices[brick_data_index * BRICK_VOLUME + current_voxel_morton];
  u32vec3 current_voxel_position = morton_decode_3(current_voxel_morton);

  // Write voxel_normal 
  uint32_t pv = palette_list.voxels[palette_index + voxel_index];
  palette_list.voxels[palette_index + voxel_index] = (pv & (0xFF00FF)) | (uint32_t((float(current_voxel_morton) / 512.0) * 255) << 2);
  
}
