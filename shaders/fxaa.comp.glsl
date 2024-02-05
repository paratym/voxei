#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout (set = 0, binding = 0, rgba8) uniform image2D img_in;
layout (set = 0, binding = 1, rgba8) uniform image2D img_out;
layout (set = 0, binding = 1) uniform Resolution {
    uvec2 resolution;
} resolution;

float rgb_to_luminance(vec3 color) {
  return sqrt(dot(color, vec3(0.299, 0.587, 0.114)));
}

void main() {
  ivec2 coord = ivec2(gl_GlobalInvocationID.xy);
  vec3 color = imageLoad(img_in, coord).rgb;

  float luminance = rgb_to_luminance(imageLoad(img_in, coord).rgb);

  float lumaDown = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x, coord.y + 1)).rgb);
  float lumaRight = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x + 1, coord.y)).rgb);
  float lumaLeft = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x - 1, coord.y)).rgb);
  float lumaUp = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x, coord.y - 1)).rgb);

  float lumaMin = min(luminance, min(lumaDown, min(lumaRight, min(lumaLeft, lumaUp))));
  float lumaMax = max(luminance, max(lumaDown, max(lumaRight, max(lumaLeft, lumaUp))));

  float lumaRange = lumaMax - lumaMin;

  const float EDGE_THRESHOLD_MIN = 0.0312;
  const float EDGE_THRESHOLD_MAX = 0.125;
  if (lumaRange < max(EDGE_THRESHOLD_MIN, EDGE_THRESHOLD_MAX * lumaMax)) {
    lumaRange = 0.0;
  }

  lumaRange *= 4.0;

  imageStore(img_out, coord, vec4(lumaRange, lumaRange, lumaRange, 1.0));
}
