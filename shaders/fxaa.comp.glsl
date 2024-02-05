#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout (set = 0, binding = 0, rgba8) uniform image2D img_in;
layout (set = 0, binding = 1, rgba8) uniform image2D img_out;
layout (set = 0, binding = 2) uniform Resolution {
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

  float lumaDownLeft = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x - 1, coord.y + 1)).rgb);
  float lumaDownRight = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x + 1, coord.y + 1)).rgb);
  float lumaUpLeft = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x - 1, coord.y - 1)).rgb);
  float lumaUpRight = rgb_to_luminance(imageLoad(img_in, ivec2(coord.x + 1, coord.y - 1)).rgb);

  float lumaDownUp = lumaDown + lumaUp;
  float lumaLeftRight = lumaLeft + lumaRight;

  float lumaLeftCorner = lumaDownLeft + lumaUpLeft;
  float lumaRightCorner = lumaDownRight + lumaUpRight;
  float lumaUpCorner = lumaUpLeft + lumaUpRight;
  float lumaDownCorner = lumaDownLeft + lumaDownRight;

  float edgeHorizonal = abs(-2.0 * lumaLeft + lumaLeftCorner) + abs(-2.0 * luminance + lumaDownUp) * 2.0 + abs(-2.0 * lumaRight + lumaRightCorner);
  float edgeVertical = abs(-2.0 * lumaUp + lumaUpCorner) + abs(-2.0 * luminance + lumaLeftRight) * 2.0 + abs(-2.0 * lumaDown + lumaDownCorner);

  bool isHorizontal = edgeHorizonal >= edgeVertical;

  float lumaMin = min(luminance, min(lumaDown, min(lumaRight, min(lumaLeft, lumaUp))));
  float lumaMax = max(luminance, max(lumaDown, max(lumaRight, max(lumaLeft, lumaUp))));

  float lumaRange = lumaMax - lumaMin;

  const float EDGE_THRESHOLD_MIN = 0.001;
  const float EDGE_THRESHOLD_MAX = 0.00001;
  if (lumaRange < max(EDGE_THRESHOLD_MIN, EDGE_THRESHOLD_MAX * lumaMax)) {
    imageStore(img_out, coord, vec4(color, 1.0));
    return;
  }

  
  float luma1 = isHorizontal ? lumaDown : lumaLeft;
  float luma2 = isHorizontal ? lumaUp : lumaRight;

  float grad1 = luma1 - luminance;
  float grad2 = luma2 - luminance;

  bool isSteepest = abs(grad1) >= abs(grad2);
  float gradScaled = max(abs(grad1), abs(grad2)) * 0.25;

  float stepLength = isHorizontal ? (1.0 / resolution.resolution.x) : (1.0 / resolution.resolution.y);
  float lumaLocalAverage = 0.0;

  if(isSteepest) {
    stepLength = -stepLength;
    lumaLocalAverage = (luma1 + luminance) * 0.5;
  } else {
    lumaLocalAverage = (luma2 + luminance) * 0.5;
  }
  lumaRange = lumaLocalAverage + gradScaled;

  color /= vec3(lumaLocalAverage);

  imageStore(img_out, coord, vec4(color, 1.0));
}
