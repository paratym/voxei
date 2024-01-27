#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#include "lib/types/voxel.glsl"
#include "lib/types/ray.glsl"

layout (set = 0, binding = 0, rgba8) uniform image2D img_output;
layout (set = 0, binding = 1) uniform Camera {
  mat4 view;
  mat4 proj;
  mat4 proj_view;
  vec2 resolution;
  float fov;
  float aspect;
  vec3 position;
} camera;

layout (set = 0, binding = 2, std430) buffer VoxelNodeSSBO {
  uint voxel_node_len;
  VoxelNode voxel_nodes[];
};

layout (set = 0, binding = 3, std430) buffer VoxelDataSSBO {
  uint voxel_data_len;
  VoxelData voxel_data[];
};
layout (set = 0, binding = 4) uniform VoxelInfo {
  vec3 bbmin;
  vec3 bbmax;
  float unit_length;
  uint grid_length;
} voxel_info;

const uint SUBDIVISIONS = 3;

#include "lib/morton.glsl"
#include "lib/voxel.glsl"
#include "lib/intersect.glsl"

vec4 calculate_lighting(vec3 p, vec3 n, vec3 rd) {
  const vec3 LIGHT_POS = vec3(2.0, 2.0, 2.0);
  vec3 light_dir = normalize(LIGHT_POS - p);
  float diffuse = max(dot(n, light_dir), 0.1);

  const vec3 BASE_COLOR = vec3(1.0, 0.4, 0.2);

  return vec4(BASE_COLOR * diffuse, 1.0);
}

float get_half_length(uint depth) {
  return 0.5 * exp2(float(SUBDIVISIONS - depth)) * voxel_info.unit_length;
}

// eg. ray_box_intersection(ray, vec3(0.0), 0)
// with subdivisions of 3 would be calculating the bounds
// of a min (-4, -4, -4) and max (4, 4, 4) box
vec2 ray_voxel_intersection(Ray ray, vec3 pos, uint depth) {
  float half_length = get_half_length(depth);
  return ray_box_intersection(ray, pos - half_length, pos + half_length);
}

// Returns the 3 digit morton code for the child voxel local position
// ray - the ray to test the direction of
// tmid - the t value of the middle of the voxel
// tc - the t values of the entry and exit of the parent voxel
uint get_child_local_morton(Ray ray, vec3 tmid, vec2 tc) {
  uint i = 0;
  if(tmid.x > tc.x) {
    i |= 1;
  }
  if(tmid.y > tc.x) {
    i |= 2;
  }
  return i;
}

vec3 get_child_position(vec3 parent_pos, uint child_local_morton, uint child_depth) {
  float half_length = get_half_length(child_depth);
  vec3 child_mask = vec3(
    (child_local_morton & 1) > 0 ? 1.0 : -1.0,
    (child_local_morton & 2) > 0 ? 1.0 : -1.0,
    (child_local_morton & 4) > 0 ? 1.0 : -1.0
  );
  return parent_pos + (child_mask * half_length);
}

struct TraceOutput {
  bool hit;
  vec3 pos;
  vec3 color;
};

TraceOutput trace(Ray ray, out vec3 pos) {
  vec2 root_t_corners = ray_box_intersection(ray, voxel_info.bbmin, voxel_info.bbmax);
  bool hit = root_t_corners.y > max(root_t_corners.x, 0.0);
  if(!hit) {
    return TraceOutput(false, vec3(0.0), vec3(0.0));
  }

  VoxelNode parent_node = voxel_nodes[voxel_node_len - 1];
  // The center position of the root node.
  vec3 root_pos = voxel_info.bbmin + ((voxel_info.bbmax - voxel_info.bbmin) * 0.5);
  vec3 root_t_mid = (root_pos - ray.origin) * ray.inv_dir;
  uint first_child_local_morton = get_child_local_morton(ray, root_t_mid, root_t_corners);
  vec3 first_child_pos = get_child_position(root_pos, first_child_local_morton, 1);

  vec3 color = vec3(0.3);
  if((first_child_local_morton & 1) > 0) {
    color = vec3(1.0, 0.0, 0.0);
  } else if((first_child_local_morton & 2) > 0) {
    color = vec3(0.0, 1.0, 0.0);
  } else if((first_child_local_morton & 4) > 0) {
    color = vec3(0.0, 0.0, 1.0);
  }
  color = vec3(root_t_corners.x / 5, 0.0, root_t_corners.y / 5);

  return TraceOutput(true, vec3(0.0), color);
}

void main() {
  vec2 coord = gl_GlobalInvocationID.xy;
  if(coord.x > camera.resolution.x || coord.y > camera.resolution.y) {
    return;
  }

  vec2 ndc = coord / camera.resolution;
  vec2 uv = vec2(ndc.x * 2.0 - 1.0, 1 - 2 * ndc.y);
  vec2 scaled_uv = vec2(uv.x * camera.aspect, uv.y) * tan(camera.fov / 2.0);

  vec3 ro = vec3(vec4(0.0, 0.0, 0.0, 1.0) * camera.view);
  vec3 rd = normalize(vec3(scaled_uv, 1.0)) * mat3(camera.view);

  vec3 pos;
  TraceOutput trace_out = trace(Ray(ro, rd, 1.0 / rd), pos);

  vec4 color = vec4(0.0, 0.0, 0.0, 1.0);

  if(trace_out.hit) {
    color = vec4(trace_out.color, 1.0);
  }
  
  imageStore(img_output, ivec2(coord), color);
}
