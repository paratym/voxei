#version 450 core
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

float distance_to_sphere(vec3 p, vec3 center, float radius) {
  return length(p - center) - radius;
}

void main() {
  vec2 coord = gl_GlobalInvocationID.xy;
  vec4 color = vec4(0.0, 1.0, 0.0, 1.0);

  if(coord.x > camera.resolution.x || coord.y > camera.resolution.y) {
    return;
  }

  vec2 ndc = coord / camera.resolution;
  vec2 uv = vec2(ndc.x * 2.0 - 1.0, 1 - 2 * ndc.y);
  vec2 scaled_uv = vec2(uv.x * camera.aspect, uv.y) * tan(camera.fov / 2.0);

  vec3 ro = camera.position;
  vec3 rd = vec3(scaled_uv, 1.0);

  float t = 0.0;
  const int MAX_STEPS = 32;
  const float MAX_DIST = 100.0;
  const float HIT_DIST = 0.001;

  for(int i = 0; i < MAX_STEPS; i++) {
    vec3 p = ro + rd * t;
    float d = distance_to_sphere(p, vec3(0.0, 0.0, 5.0), 1.0);

    if(d < HIT_DIST) {
      color = vec4(1.0, 0.0, 0.0, 1.0);
      break;
    }
    if(t > MAX_DIST) {
      break;
    }

    t += d;
  }

  
  imageStore(img_output, ivec2(coord), color);
}
