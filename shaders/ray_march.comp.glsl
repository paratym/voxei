layout (local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

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
const float EPSILON = 0.01;

TraceWorldOut trace_vox_world(Ray ray) {
  VoxelWorldInfo info = get_buffer(push_constants.voxel_world_info_id, VoxelWorldInfo);
  ChunkOccupancyGrid chunk_occupancy_grid = get_buffer(info.chunk_occupancy_grid_buffer, ChunkOccupancyGrid);
  BrickIndicesGrid brick_indices_grid = get_buffer(info.brick_indices_grid_buffer, BrickIndicesGrid);
  BrickDataList brick_data_list = get_buffer(info.brick_data_buffer, BrickDataList);
  BrickRequestList brick_request_list = get_buffer(info.brick_request_list_buffer, BrickRequestList);

  float dyn_world_world_side_length = info.chunk_side_length * CHUNK_WORLD_LENGTH;
  float dyn_world_world_half_length = dyn_world_world_side_length / 2;

  vec3 dyn_world_center = info.chunk_center * CHUNK_WORLD_LENGTH;
  AABB dyn_world_aabb = AABB(dyn_world_center - dyn_world_world_half_length, dyn_world_center + dyn_world_world_half_length);

  RayAABBIntersection intersection = ray_aabb_intersection(ray, dyn_world_aabb);
  if(!intersection.hit) {
    return trace_world_out_miss();
  }

  vec3 enter_pos = ray.origin + ray.dir * (intersection.tenter + EPSILON);
  vec3 normalized_world_pos = ((enter_pos - dyn_world_center) + dyn_world_world_half_length) / dyn_world_world_side_length;
  uint32_t dyn_world_voxel_side_length = info.chunk_side_length * CHUNK_VOXEL_LENGTH;

  i32vec3 chunk_local = i32vec3(floor(normalized_world_pos * info.chunk_side_length));
  uint32_t chunk_morton = morton_encode_3(chunk_local.x, chunk_local.y, chunk_local.z);
  uint32_t chunk_status = ((chunk_occupancy_grid.grid[chunk_morton >> 3] >> ((chunk_morton & 7) * 2)) & 3);
  // debugPrintfEXT("dyn_world_chunk_local: %u %u %u\n", dyn_world_chunk_local.x, dyn_world_chunk_local.y, dyn_world_chunk_local.z);

  for (uint32_t i = 0; i < 100; i++) {
    vec3 chunk_min = (i32vec3(chunk_local) - int(info.chunk_half_length)) * CHUNK_WORLD_LENGTH ;
    vec3 chunk_max = chunk_min + CHUNK_WORLD_LENGTH;
    AABB chunk_aabb = AABB(chunk_min, chunk_max);
    RayAABBIntersection chunk_intersection = ray_aabb_intersection(ray, chunk_aabb);

    if(chunk_local.x < 0 || chunk_local.y < 0 || chunk_local.z < 0 || chunk_local.x >= info.chunk_side_length || chunk_local.y >= info.chunk_side_length || chunk_local.z >= info.chunk_side_length) {
      return trace_world_out_hit(vec3(0,(i+1)/40.0,0));
    }
    if(chunk_status == 0) {
      // Handle unloaded chunk case
    } else if(chunk_status == 1) {
      // handle it is loading rn.
    } else {
      // Show that chunk.
      vec3 brick_ray_pos = ray.origin + (ray.dir * (chunk_intersection.tenter + EPSILON));
      vec3 normalized_brick_pos = ((brick_ray_pos - dyn_world_center) + dyn_world_world_half_length) / dyn_world_world_side_length;
      u32vec3 brick_enter_local = u32vec3(floor(normalized_brick_pos * info.chunk_side_length * CHUNK_LENGTH));
      Ray brick_ray = Ray(brick_ray_pos - chunk_min, ray.dir, ray.inv_dir);

      i32vec3 lb = i32vec3(brick_enter_local) % i32vec3(CHUNK_LENGTH);
      u32vec3 brick_chunk_root = u32vec3(chunk_local) * u32vec3(CHUNK_LENGTH);

      while(lb.x >= 0 && lb.y >= 0 && lb.z >= 0 && lb.x < CHUNK_LENGTH && lb.y < CHUNK_LENGTH && lb.z < CHUNK_LENGTH) {
        u32vec3 wb = u32vec3(lb) + brick_chunk_root;
        uint32_t brick_morton = morton_encode_3(wb.x, wb.y, wb.z);
        uint32_t brick_data = brick_indices_grid.grid[brick_morton];
        uint32_t brick_index = brick_data & 0x3FFFFFFF;
        uint32_t brick_status = brick_data >> 30;

        vec3 brick_min = lb * BRICK_WORLD_LENGTH;
        vec3 brick_max = brick_min + BRICK_WORLD_LENGTH;
        AABB brick_aabb = AABB(brick_min, brick_max);
        RayAABBIntersection brick_intersection = ray_aabb_intersection(brick_ray, brick_aabb);
        if(brick_status == 0) {
          // brick is unloaded
        } else if(brick_status == 1) {
          // brick is loading
        } else if(brick_status == 2) {
          vec3 voxel_ray_pos = brick_ray.origin + (ray.dir * (brick_intersection.tenter + EPSILON));
          if(point_aabb_intersection(voxel_ray_pos, brick_aabb) == true) {
            i32vec3 vl = i32vec3(i32vec3(floor(voxel_ray_pos)) % BRICK_LENGTH);
            Ray voxel_ray = Ray(voxel_ray_pos - brick_min, ray.dir, ray.inv_dir);
            if(brick_index >= 20000 ) {
              return trace_world_out_hit(vec3(1,0,0));
            }
          
            BrickData b_data = brick_data_list.data[brick_index];
            while(vl.x >= 0 && vl.y >= 0 && vl.z >= 0 && vl.x < BRICK_LENGTH && vl.y < BRICK_LENGTH && vl.z < BRICK_LENGTH) {
              uint32_t voxel_morton = morton_encode_3(vl.x, vl.y, vl.z);
              uint32_t voxel_status = b_data.voxel_mask[voxel_morton >> 3] & (1 << (voxel_morton & 7));
              if(voxel_status > 0) {
                return trace_world_out_hit(vec3(0,0,(vl.x + vl.y + vl.z) / float(BRICK_LENGTH * 3)));
              }
              vec3 voxel_min = vl * VOXEL_WORLD_LENGTH;
              vec3 voxel_max = voxel_min + VOXEL_WORLD_LENGTH;
              AABB voxel_aabb = AABB(voxel_min, voxel_max);
              RayAABBIntersection voxel_intersection = ray_aabb_intersection(voxel_ray, voxel_aabb);
              u32vec3 exit_axes = u32vec3(
                voxel_intersection.tmax.x == voxel_intersection.texit ? 1 : 0,
                voxel_intersection.tmax.y == voxel_intersection.texit ? 1 : 0,
                voxel_intersection.tmax.z == voxel_intersection.texit ? 1 : 0
              );
              vec3 advance_axes = vec3(exit_axes) * sign(ray.dir);
              vl += i32vec3(advance_axes);
            }
          }
        }

        u32vec3 exit_axes = u32vec3(
          brick_intersection.tmax.x == brick_intersection.texit ? 1 : 0,
          brick_intersection.tmax.y == brick_intersection.texit ? 1 : 0,
          brick_intersection.tmax.z == brick_intersection.texit ? 1 : 0
        );

        vec3 advance_axes = vec3(exit_axes) * sign(ray.dir);
        lb += i32vec3(advance_axes);
      }
    }

    u32vec3 exit_axes = u32vec3(
      chunk_intersection.tmax.x == chunk_intersection.texit ? 1 : 0,
      chunk_intersection.tmax.y == chunk_intersection.texit ? 1 : 0,
      chunk_intersection.tmax.z == chunk_intersection.texit ? 1 : 0
    );
    vec3 advance_axes = vec3(exit_axes) * sign(ray.dir);
    chunk_local += i32vec3(advance_axes);
    chunk_morton = morton_encode_3(chunk_local.x, chunk_local.y, chunk_local.z);
    chunk_status = ((chunk_occupancy_grid.grid[chunk_morton >> 3] >> ((chunk_morton & 7) * 2)) & 3);
  }

  return trace_world_out_hit(vec3(enter_pos));
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
