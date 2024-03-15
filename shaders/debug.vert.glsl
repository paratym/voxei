layout(location = 0) in vec3 in_position;

#include "lib/types.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId camera_id;
} push_constants;

const float vertices[9] = float[9](
  -2, -2, 0.2,
  2, -2, 0.2,
  0, 2, 0.2
);


void main() {
  Camera camera = get_buffer(push_constants.camera_id, Camera);
  vec3 position = vec3(vertices[gl_VertexIndex * 3], vertices[gl_VertexIndex * 3 + 1], vertices[gl_VertexIndex * 3 + 2]);
  gl_Position = camera.projView * vec4(in_position, 1.0);
}
