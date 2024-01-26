#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#include "lib/types/voxel.glsl"

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

#include "lib/morton.glsl"
#include "lib/voxel.glsl"

vec4 calculate_lighting(vec3 p, vec3 n, vec3 rd) {
  const vec3 LIGHT_POS = vec3(2.0, 2.0, 2.0);
  vec3 light_dir = normalize(LIGHT_POS - p);
  float diffuse = max(dot(n, light_dir), 0.1);

  const vec3 BASE_COLOR = vec3(1.0, 0.4, 0.2);

  return vec4(BASE_COLOR * diffuse, 1.0);
}

vec4 ray_march(vec3 ro, vec3 rd) {
  vec3 t0 = (vec3(0, 0, 0) - ro) / rd;
  vec3 t1 = (vec3(2, 2, 2) - ro) / rd;
  vec3 tmin = min(t0, t1);
  vec3 tmax = max(t0, t1);
  vec2 traverse = max(tmin.xx, tmin.yz);
  float tenter = max(traverse.x, traverse.y);
  traverse = min(tmax.xx, tmax.yz);
  float texit = min(traverse.x, traverse.y);
  vec3 box = vec3(float(texit > max(tenter, 0.0)), tenter, texit);

  if(box.x == 0.0) {
    return vec4(0.0, 0.0, 0.0, 1.0);
  }

  ro += tenter * rd;

  return vec4(1.0, 0.0, 0.0, 1.0);
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

  vec4 color = ray_march(ro, rd);
  
  imageStore(img_output, ivec2(coord), color);
}
