u32vec3 world_to_voxel_local(vec3 world_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
  float voxel_side_length = chunk_render_distance * CHUNK_LENGTH * BRICK_LENGTH * 2;
  float voxel_unit_half_length = (voxel_side_length / 2) * VOXEL_UNIT_LENGTH;

  vec3 world_center = vec3(chunk_center) * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;

  vec3 world_min = world_center + vec3(-voxel_unit_half_length);
  vec3 unit_world_pos = (world_pos - world_min) / (voxel_unit_half_length * 2);
  u32vec3 local_voxel_pos = u32vec3(floor(unit_world_pos * voxel_side_length));

  return local_voxel_pos;
}

u32vec3 voxel_local_to_chunk_local(u32vec3 local_voxel_pos) {
  return local_voxel_pos / uint32_t(CHUNK_LENGTH * BRICK_LENGTH);
}

u32vec3 voxel_local_to_brick_local(u32vec3 local_voxel_pos) {
  return local_voxel_pos / uint32_t(BRICK_LENGTH);
}

uint32_t voxel_local_to_morton(u32vec3 local_voxel_pos, uint32_t chunk_render_distance) {
  float voxel_side_length = chunk_render_distance * CHUNK_LENGTH * BRICK_LENGTH * 2;

  uint32_t morton = 0;
  uint32_t depth = uint32_t(log2(voxel_side_length)); 
  for (int i = 0; i < depth; i++) {
    morton |= (local_voxel_pos.x & (1 << i)) << (i * 2);
    morton |= (local_voxel_pos.y & (1 << i)) << (i * 2 + 1);
    morton |= (local_voxel_pos.z & (1 << i)) << (i * 2 + 2);
  }

  return morton;
}

AABB chunk_local_to_aabb(u32vec3 chunk_local_pos, i32vec3 chunk_center, uint32_t chunk_render_distance) {
  vec3 chunk_world_min = (chunk_center + (i32vec3(chunk_local_pos) - int(chunk_render_distance))) * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;
  //debugPrintfEXT("Chunk world min: %f %f %f\n", chunk_world_min.x, chunk_world_min.y, chunk_world_min.z);
  vec3 chunk_world_max = chunk_world_min + vec3(CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH);
  // debugPrintfEXT("Chunk world max: %f %f %f\n", chunk_world_max.x, chunk_world_max.y, chunk_world_max.z);
  

  return AABB(chunk_world_min, chunk_world_max);
}
