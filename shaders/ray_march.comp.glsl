layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

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

const uint32_t MARCH_ITERATIONS = 2048;
const float MARCH_EPSILON = 0.001;

TraceWorldOut trace_vox_world(Ray ray) {
  VoxelWorldInfo info = get_buffer(push_constants.voxel_world_info_id, VoxelWorldInfo);
  ChunkOccupancyGrid chunk_occupancy_grid = get_buffer(info.chunk_occupancy_grid_buffer, ChunkOccupancyGrid);
  float half_length = info.chunk_render_distance * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;

  vec3 world_center = info.chunk_center * CHUNK_LENGTH * BRICK_LENGTH * VOXEL_UNIT_LENGTH;
  AABB world_aabb = AABB(world_center + vec3(-half_length), world_center + vec3(half_length));

  RayAABBIntersection intersection = ray_aabb_intersection(ray, world_aabb);
  if(!intersection.hit) {
    return trace_world_out_miss();
  }

  vec3 curr_pos = ray.origin + ray.dir * (intersection.tenter + MARCH_EPSILON);
  uint32_t max_iterations = uint32_t(info.chunk_render_distance * 2 * CHUNK_LENGTH * BRICK_LENGTH);
  for(int i = 0; i < max_iterations; i++) {
    if(!point_aabb_intersection(curr_pos, world_aabb)) {
      return trace_world_out_miss();
    }

    u32vec3 curr_voxel_local = world_to_voxel_local(curr_pos, info.chunk_center, info.chunk_render_distance);

    uint32_t morton = voxel_local_to_morton(curr_voxel_local, info.chunk_render_distance);
    uint32_t chunk_morton = morton >> uint32_t((BRICK_DEPTH + CHUNK_DEPTH) * 3);
    if((chunk_occupancy_grid.grid[chunk_morton >> 3] & (1 << (chunk_morton & 7))) > 0) {
      uint32_t brick_morton = morton >> uint32_t((BRICK_DEPTH) * 3);

      // Go into the brick level
      return trace_world_out_hit(vec3(0.2, 0.2, 1.0));
    } else {
      // Skip chunk since it is empty
      u32vec3 chunk_local = voxel_local_to_chunk_local(curr_voxel_local);
      AABB chunk_aabb = chunk_local_to_aabb(chunk_local, info.chunk_center, info.chunk_render_distance);
      RayAABBIntersection chunk_intersection = ray_aabb_intersection(ray, chunk_aabb);

      curr_pos = ray.origin + ray.dir * (chunk_intersection.texit + MARCH_EPSILON);
    }
  }

  return trace_world_out_hit(vec3(1.0, 0.2, 0.2));
}

void main() {
  Camera camera = get_buffer(push_constants.camera_id, Camera);

  vec2 coord = gl_GlobalInvocationID.xy;
  if(coord.x > camera.resolution.x || coord.y > camera.resolution.y) {
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
