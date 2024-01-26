float distance_to_sphere(vec3 p, vec3 center, float radius) {
  return length(p - center) - radius;
}

uint calculate_voxel_index(vec3 p) {
  return 0;
}
