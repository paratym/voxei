#version 450

#include "lib/voxel.glsl"

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

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
layout (set = 0, binding = 2) buffer voxel_ssbo {
  uint num_length;
  VoxelData voxels[];
};

vec4 calculate_lighting(vec3 p, vec3 n, vec3 rd) {
  const vec3 LIGHT_POS = vec3(2.0, 2.0, 2.0);
  vec3 light_dir = normalize(LIGHT_POS - p);
  float diffuse = max(dot(n, light_dir), 0.1);

  const vec3 BASE_COLOR = vec3(1.0, 0.4, 0.2);

  return vec4(BASE_COLOR * diffuse, 1.0);
}

vec4 ray_march(vec3 ro, vec3 rd) {
  float t = 0.0;
  const int MAX_STEPS = 32;
  const float MAX_DIST = 100.0;
  const float HIT_DIST = 0.001;

  const vec3 SPHERE_CENTER = vec3(0.0, 0.0, 5.0);
  const float SPHERE_RADIUS = 1.0;

  for(int i = 0; i < MAX_STEPS; i++) {
    vec3 p = ro + rd * t;
    float d = distance_to_sphere(p, SPHERE_CENTER, SPHERE_RADIUS);

    if(d < HIT_DIST) {
      vec3 n = normalize(p - SPHERE_CENTER);

      return calculate_lighting(p, n, rd);
    }
    if(t > MAX_DIST) {
      break;
    }

    t += d;
  }

  return vec4(0.0, 0.0, 0.0, 1.0);
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
