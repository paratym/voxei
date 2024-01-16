#version 450 core
layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout (set = 0, binding = 0, rgba8) uniform image2D img_output;
layout (set = 0, binding = 1) uniform UBuf {
  vec2 resolution;
};
layout (set = 0, binding = 2) uniform Camera {
  mat4 view;
  mat4 proj;
  mat4 proj_view;
};

void main() {
  vec2 coord = gl_GlobalInvocationID.xy;
  vec4 color = vec4(0.0, 1.0, 0.0, 1.0);

  if(coord.x > resolution.x || coord.y > resolution.y) {
    return;
  }

  imageStore(img_output, ivec2(coord), color);
}
