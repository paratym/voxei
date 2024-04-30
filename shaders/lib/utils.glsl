// u32vec3 world_to_voxel_local(vec3 world_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
//   float voxel_side_length = chunk_render_distance * CHUNK_LENGTH * BRICK_LENGTH * 2;
//   float voxel_unit_half_length = (voxel_side_length / 2) * VOXEL_UNIT_LENGTH;
// 
//   vec3 world_center = vec3(chunk_center) * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;
// 
//   vec3 world_min = world_center + vec3(-voxel_unit_half_length);
//   vec3 unit_world_pos = (world_pos - world_min) / (voxel_unit_half_length * 2);
//   u32vec3 local_voxel_pos = u32vec3(floor(unit_world_pos * voxel_side_length));
// 
//   return local_voxel_pos;
// }
// 
// u32vec3 voxel_local_to_chunk_local(u32vec3 local_voxel_pos) {
//   return local_voxel_pos / uint32_t(CHUNK_LENGTH * BRICK_LENGTH);
// }
// 
// u32vec3 voxel_local_to_brick_local(u32vec3 local_voxel_pos) {
//   return local_voxel_pos / uint32_t(BRICK_LENGTH);
// }
// 
// uint32_t local_to_morton(u32vec3 local_voxel_pos, uint32_t side_length) {
//   uint32_t morton = 0;
//   uint32_t depth = uint32_t(log2(side_length)); 
//   for (int i = 0; i < depth; i++) {
//     morton |= (local_voxel_pos.x & (1 << i)) << (i * 2);
//     morton |= (local_voxel_pos.y & (1 << i)) << (i * 2 + 1);
//     morton |= (local_voxel_pos.z & (1 << i)) << (i * 2 + 2);
//   }
// 
//   return morton;
// }
// 
// AABB chunk_local_to_aabb(u32vec3 chunk_local_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
//   vec3 translated_chunk_local_pos = chunk_center + (i32vec3(chunk_local_pos) - int(chunk_render_distance));
//   vec3 chunk_world_min = translated_chunk_local_pos * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;
//   //debugPrintfEXT("Chunk world min: %f %f %f\n", chunk_world_min.x, chunk_world_min.y, chunk_world_min.z);
//   vec3 chunk_world_max = chunk_world_min + vec3(CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH);
//   // debugPrintfEXT("Chunk world max: %f %f %f\n", chunk_world_max.x, chunk_world_max.y, chunk_world_max.z);
//   
// 
//   return AABB(chunk_world_min, chunk_world_max);
// }
// 
// AABB brick_local_to_aabb(u32vec3 brick_local_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
//   vec3 translated_brick_local_pos = (chunk_center * CHUNK_LENGTH) + (i32vec3(brick_local_pos) - int(chunk_render_distance * CHUNK_LENGTH));
//   vec3 brick_world_min = translated_brick_local_pos * BRICK_LENGTH * VOXEL_UNIT_LENGTH;
//   //debugPrintfEXT("Brick world min: %f %f %f\n", brick_world_min.x, brick_world_min.y, brick_world_min.z);
//   vec3 brick_world_max = brick_world_min + vec3(BRICK_LENGTH * VOXEL_UNIT_LENGTH);
//   //debugPrintfEXT("Brick world max: %f %f %f\n", brick_world_max.x, brick_world_max.y, brick_world_max.z);
// 
//   return AABB(brick_world_min, brick_world_max);
// }
// 
// AABB voxel_local_to_aabb(u32vec3 voxel_local_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
//   vec3 translated_voxel_local_pos = (chunk_center * CHUNK_LENGTH * BRICK_LENGTH) + (i32vec3(voxel_local_pos) - int(chunk_render_distance * CHUNK_LENGTH * BRICK_LENGTH));
//   vec3 voxel_world_min = translated_voxel_local_pos * VOXEL_UNIT_LENGTH;
//   //debugPrintfEXT("Voxel world min: %f %f %f\n", voxel_world_min.x, voxel_world_min.y, voxel_world_min.z);
//   vec3 voxel_world_max = voxel_world_min + vec3(VOXEL_UNIT_LENGTH);
//   //debugPrintfEXT("Voxel world max: %f %f %f\n", voxel_world_max.x, voxel_world_max.y, voxel_world_max.z);
// 
//   return AABB(voxel_world_min, voxel_world_max);
// }

uint octahedral_8_encode(vec3 nor) {
    nor /= (abs(nor.x) + abs(nor.y) + abs(nor.z));
    nor.xy = (nor.z >= 0.0) ? nor.xy : (1.0-abs(nor.yx))*sign(nor.xy);
    vec2 v = 0.5 + 0.5*nor.xy;

    uvec2 d = uvec2(floor(v*15.0+0.5));
    return (d.y<<4)|d.x;
}

vec3 octahedral_8_decode(uint data) {
    uvec2 d = uvec2(data, data>>4) & 0xf;
    vec2 v = vec2(d)/15.0;
    
    v = -1.0 + 2.0*v;
    // Rune Stubbe's version, much faster than original
    vec3 nor = vec3(v, 1.0 - abs(v.x) - abs(v.y));
    float t = max(-nor.z,0.0);
    nor.x += (nor.x>0.0)?-t:t;
    nor.y += (nor.y>0.0)?-t:t;
    return normalize( nor );
}

VoxelMaterial unpack_voxel(uint32_t voxel) {
  uint32_t albedo_u = voxel & 0xffffff;
  uint32_t octa_norm = (voxel >> 18) & 0xff;
  vec3 norm = octahedral_8_decode(octa_norm);
  vec3 albedo = vec3(float((albedo_u >> 12) & 0x3f), float((albedo_u >> 6) & 0x3f), float(albedo_u & 0x3f)) / 63.0;
  return VoxelMaterial(albedo, norm);
}

// Split first 10 bits by inserting two 0s to the left of each bit.
uint32_t morton_split_by_2(uint32_t x) {
  uint32_t y = x & 0x000003ff; //      00000000000000000000001111111111
  y = (y | (y << 16)) & 0x030000ff; // 00000011000000000000000011111111
  y = (y | (y << 8)) & 0x0300f00f; //  00000011000000001111000000001111
  y = (y | (y << 4)) & 0x030c30c3; //  00000011000011000011000011000011
  y = (y | (y << 2)) & 0x09249249; //  00001001001001001001001001001001
  return y;
}

uint32_t morton_encode_3(uint32_t x, uint32_t y, uint32_t z) {
  return morton_split_by_2(x) | (morton_split_by_2(y) << 1) | (morton_split_by_2(z) << 2);
}

uint32_t morton_compact_by_1(uint32_t x) {
  uint32_t y = x & 0x55555555; //      01010101010101010101010101010101
  y = (y | (y >> 1)) & 0x33333333; //  00110011001100110011001100110011
  y = (y | (y >> 2)) & 0x0f0f0f0f; //  00001111000011110000111100001111
  y = (y | (y >> 4)) & 0x00ff00ff; //  00000000111111110000000011111111
  y = (y | (y >> 8)) & 0x0000ffff; //  00000000000000001111111111111111
  return y;
}

u32vec2 morton_decode_2(uint32_t morton) {
  return u32vec2(morton_compact_by_1(morton), morton_compact_by_1(morton >> 1));
}

uint32_t morton_compact_by_2(uint32_t x) {
  uint32_t y = x & 0x09249249; //      00001001001001001001001001001001
  y = (y | (y >> 2)) & 0x030c30c3; //  00000011000011000011000011000011
  y = (y | (y >> 4)) & 0x0300f00f; //  00000011000000001111000000001111
  y = (y | (y >> 8)) & 0x030000ff; //  00000011000000000000000011111111
  y = (y | (y >> 16)) & 0x000003ff; // 00000000000000000000001111111111
  return y;
}

u32vec3 morton_decode_3(uint32_t morton) {
  return u32vec3(morton_compact_by_2(morton), morton_compact_by_2(morton >> 1), morton_compact_by_2(morton >> 2));
}

