struct VoxelData {
  uint material_id;
};

float distance_to_sphere(vec3 p, vec3 center, float radius) {
  return length(p - center) - radius;
}
