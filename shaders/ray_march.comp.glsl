layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

#include "lib/constants.glsl"
#include "lib/types.glsl"
#include "lib/intersect.glsl"
#include "lib/utils.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId backbuffer_id;
  ResourceId camera_id;
  ResourceId voxel_world_info_id;
} push_constants;

struct TraceWorldOut {
  vec3 color;
  bool hit;
};

TraceWorldOut trace_world_out_miss() {
  return TraceWorldOut(vec3(0, 0, 0), false);
}

TraceWorldOut trace_world_out_hit(vec3 color) {
  return TraceWorldOut(color, true);
}

const float EPSILON = 0.000001;
const vec3 LIGHT_DIR = normalize(vec3(-0.5,-1,1));
const vec3 LIGHT_POS = vec3(100, 100, 0);

TraceWorldOut trace_brick(Ray ray, uint32_t data_index, vec3 normal, in VoxelWorldInfo info, vec3 brick_world_pos) {
  if(data_index >= 500000) {
    return trace_world_out_hit(vec3(0.6, 0.1, 0.1));
  }

  BrickDataList brick_data_list = get_buffer(info.brick_data_buffer, BrickDataList);
  BrickData brick_data = brick_data_list.data[data_index];
  BrickPaletteList brick_palette_list = get_buffer(info.brick_palette_list_buffer, BrickPaletteList);
  BrickPaletteIndicesList brick_palette_indices_list = get_buffer(info.brick_palette_indices_list_buffer, BrickPaletteIndicesList);

  i32vec3 map_pos = i32vec3(floor(ray.origin));
  i32vec3 step_axes = i32vec3(sign(ray.dir));
  // The amount to increment t (ray distance) to increment one unit on each respected axis.
  vec3 t_unit_delta = abs(ray.inv_dir);
  // The current t value to intersect each axis depending on the ray direction.
  vec3 curr_t = (sign(ray.dir) * (map_pos - ray.origin) + (sign(ray.dir) * 0.5) + 0.5) * t_unit_delta;
  vec3 last_t = vec3(0.0);

  while(map_pos.x >= 0 && map_pos.y >= 0 && map_pos.z >= 0 && 
    map_pos.x < BRICK_LENGTH && map_pos.y < BRICK_LENGTH && map_pos.z < BRICK_LENGTH) {
    uint32_t voxel_morton = morton_encode_3(map_pos.x, map_pos.y, map_pos.z);
    uint32_t voxel_status = brick_data.voxel_mask[voxel_morton >> 3] & (1 << (voxel_morton & 7));
    if(voxel_status > 0) {
      uint32_t palette_index = brick_data.palette_index & 0x3FFFFFFF;
      uint32_t palette_size = brick_data.palette_index >> 30;
      uint32_t voxel_index = brick_palette_indices_list.indices[(data_index * BRICK_VOLUME + voxel_morton)];
      uint32_t packed_voxel = brick_palette_list.voxels[palette_index + voxel_index];
      VoxelMaterial mat = unpack_voxel(packed_voxel);

      vec3 voxel_world_pos = brick_world_pos + vec3(map_pos) * VOXEL_WORLD_LENGTH;
      vec3 to_light = normalize(LIGHT_POS - voxel_world_pos);
      float to_light_dist = length(to_light);
      float atten = 1.0 / (to_light_dist * to_light_dist);
      // half lambert
      float dotl = max(dot(mat.normal, -LIGHT_DIR), 0.1);
      float diff = pow(dotl * 0.5 + 0.5, 5.0);
      return trace_world_out_hit(mat.albedo * dotl);
    }

    bvec3 mask = lessThanEqual(curr_t.xyz, min(curr_t.yzx, curr_t.zxy));
    last_t = curr_t;
    curr_t += vec3(mask) * t_unit_delta;
    map_pos += i32vec3(mask) * step_axes;
  }

  return trace_world_out_miss();
}

TraceWorldOut trace_chunk(Ray ray, i32vec3 chunk_local, u32vec3 chunk_translated_local, vec3 normal, in VoxelWorldInfo info, vec3 dyn_world_min) { 
  BrickIndicesGrid brick_grid = get_buffer(info.brick_indices_grid_buffer, BrickIndicesGrid);

  i32vec3 map_pos = i32vec3(floor(ray.origin));
  i32vec3 step_axes = i32vec3(sign(ray.dir));
  // The amount to increment t (ray distance) to increment one unit on each respected axis.
  vec3 t_unit_delta = abs(ray.inv_dir);
  // The current t value to intersect each axis depending on the ray direction.
  vec3 curr_t = (sign(ray.dir) * (map_pos - ray.origin) + (sign(ray.dir) * 0.5) + 0.5) * t_unit_delta;
  vec3 last_t = vec3(0.0);

  while(map_pos.x >= 0 && map_pos.y >= 0 && map_pos.z >= 0 && 
    map_pos.x < CHUNK_LENGTH && map_pos.y < CHUNK_LENGTH && map_pos.z < CHUNK_LENGTH) {
    u32vec3 translated_brick_pos = chunk_translated_local * CHUNK_LENGTH + u32vec3(map_pos);
    uint32_t brick_morton = morton_encode_3(translated_brick_pos.x, translated_brick_pos.y, translated_brick_pos.z);
    uint32_t brick_index = brick_grid.grid[brick_morton];
    uint32_t brick_status = brick_index >> 30;
    if (brick_status == 2) {
      uint32_t data_index = brick_index & 0x3FFFFFFF;

      vec3 vox_normal = vec3(lessThanEqual(last_t.xyz, min(last_t.yzx, last_t.zxy))) * -step_axes;
      normal = (last_t.x + last_t.y + last_t.z) == 0.0 ? normal : vox_normal;

      vec3 brick_enter_pos = ray.origin + ray.dir * (min(min(last_t.x, last_t.y), last_t.z));
      Ray brick_local_ray = Ray((clamp(brick_enter_pos - map_pos, EPSILON, 1.0 - EPSILON)) * BRICK_LENGTH, ray.dir, ray.inv_dir);
      vec3 chunk_world_pos = dyn_world_min + vec3(chunk_local) * CHUNK_WORLD_LENGTH;
      vec3 brick_world_pos = chunk_world_pos + vec3(map_pos) * BRICK_WORLD_LENGTH;
      TraceWorldOut brick_result = trace_brick(brick_local_ray, data_index, normal, info, brick_world_pos);
      if(brick_result.hit) {
        return brick_result;
      }
    }

    bvec3 mask = lessThanEqual(curr_t.xyz, min(curr_t.yzx, curr_t.zxy));
    last_t = curr_t;
    curr_t += vec3(mask) * t_unit_delta;
    map_pos += i32vec3(mask) * step_axes;
  }
  return trace_world_out_miss();
}

TraceWorldOut trace_vox_world(Ray ray) {
  VoxelWorldInfo info = get_buffer(push_constants.voxel_world_info_id, VoxelWorldInfo);
  ChunkOccupancyGrid chunk_occupancy_grid = get_buffer(info.chunk_occupancy_grid_buffer, ChunkOccupancyGrid);

  float dyn_world_world_side_length = info.chunk_side_length * CHUNK_WORLD_LENGTH;
  float dyn_world_world_half_length = dyn_world_world_side_length / 2;

  vec3 dyn_world_center = info.chunk_center * CHUNK_WORLD_LENGTH;
  AABB dyn_world_aabb = AABB(dyn_world_center - dyn_world_world_half_length, dyn_world_center + dyn_world_world_half_length);

  RayAABBIntersection intersection = ray_aabb_intersection(ray, dyn_world_aabb);
  if(!intersection.hit) {
    return trace_world_out_miss();
  }

  vec3 enter_pos = ray.origin + ray.dir * (intersection.tenter);
  vec3 normalized_world_pos = ((enter_pos - dyn_world_center) + dyn_world_world_half_length) / dyn_world_world_side_length;

  // Transform the ray to world-chunk space
  ray = Ray(normalized_world_pos * info.chunk_side_length, ray.dir, ray.inv_dir);
  i32vec3 map_pos = i32vec3(floor(normalized_world_pos * info.chunk_side_length));
  i32vec3 step_axes = i32vec3(sign(ray.dir));
  // The amount to increment t (ray distance) to increment one unit on each respected axis.
  vec3 t_unit_delta = abs(ray.inv_dir);
  // The current t value to intersect each axis depending on the ray direction.
  vec3 curr_t = (sign(ray.dir) * (map_pos - ray.origin) + (sign(ray.dir) * 0.5) + 0.5) * t_unit_delta;
  vec3 last_t = vec3(0.0);

  // debugPrintfEXT("dyn_world_chunk_local: %u %u %u\n", dyn_world_chunk_local.x, dyn_world_chunk_local.y, dyn_world_chunk_local.z);

  while(map_pos.x >= 0 && map_pos.y >= 0 && map_pos.z >= 0 && map_pos.x < info.chunk_side_length && map_pos.y < info.chunk_side_length && map_pos.z < info.chunk_side_length) {

    u32vec3 translated_map_pos = u32vec3((map_pos + i32vec3(info.chunk_side_length) + info.chunk_translation) % info.chunk_side_length);
    uint32_t chunk_morton = morton_encode_3(translated_map_pos.x, translated_map_pos.y, translated_map_pos.z);
    uint32_t chunk_status = ((chunk_occupancy_grid.grid[chunk_morton >> 3] >> (chunk_morton & 7)) & 1);
    if(chunk_status == 1) {
      vec3 chunk_enter_pos = ray.origin + ray.dir * (min(min(last_t.x, last_t.y), last_t.z));
      Ray brick_local_ray = Ray((clamp(chunk_enter_pos - map_pos, EPSILON, 1.0 - EPSILON)) * CHUNK_LENGTH, ray.dir, ray.inv_dir);
      vec3 normal = vec3(lessThanEqual(last_t.xyz, min(last_t.yzx, last_t.zxy))) * -step_axes;
      TraceWorldOut chunk_result = trace_chunk(brick_local_ray, map_pos, translated_map_pos, normal, info, dyn_world_aabb.min);
      if(chunk_result.hit) {
        return chunk_result;
      }
    }

    bvec3 mask = lessThanEqual(curr_t.xyz, min(curr_t.yzx, curr_t.zxy));
    last_t = curr_t;
    curr_t += vec3(mask) * t_unit_delta;
    map_pos += i32vec3(mask) * step_axes;
  }

  return trace_world_out_hit(vec3(0,0.5,0.8));
}

void main() {
  Camera camera = get_buffer(push_constants.camera_id, Camera);

  vec2 coord = gl_GlobalInvocationID.xy;
  if(coord.x > camera.resolution.x || coord.y > camera.resolution.y) {
    return;
  }

  vec3 crosshair_color = vec3(0.3);
  float crosshair_thickness = 1.5;
  float crosshair_length = 12.0;
  vec2 midpoint = vec2(camera.resolution.x / 2.0, camera.resolution.y / 2.0);
  if((abs(coord.x - midpoint.x) < crosshair_thickness && abs(coord.y - midpoint.y) < crosshair_length) || 
    (abs(coord.y - midpoint.y) < crosshair_thickness && abs(coord.x - midpoint.x) < crosshair_length)) {
    imageStore(get_storage_image(push_constants.backbuffer_id), ivec2(coord), vec4(crosshair_color.xyz, 1.0));
    return;
  }

  vec2 ndc = coord / camera.resolution;
  vec2 uv = vec2(ndc.x * 2.0 - 1.0, 1 - 2 * ndc.y);
  vec2 scaled_uv = vec2(uv.x * camera.aspect, uv.y) * tan(camera.fov / 2.0);

  vec3 ro = vec3(vec4(0.0, 0.0, 0.0, 1.0) * camera.transform);
  vec3 rd = normalize(vec3(scaled_uv, 1.0)) * mat3(camera.transform);
  Ray ray = Ray(ro, rd, 1.0 / rd);

  TraceWorldOut trace_world_out = trace_vox_world(ray);

  vec3 color = vec3(0.0);
  if(trace_world_out.hit) {
    color = trace_world_out.color;
  }

  imageStore(get_storage_image(push_constants.backbuffer_id), ivec2(coord), vec4(color, 1.0));
}
