uint32_t voxel_ray_child_morton(Ray ray, vec3 center_pos, float tenter) {
  vec3 t_mid = (center_pos - ray.origin) * ray.inv_dir;
  uint32_t morton = 0;

  morton |= uint32_t(step(tenter, t_mid.x) * step(ray.dir.x, 0.0));
  morton |= uint32_t(step(t_mid.x, tenter) * step(0.0, ray.dir.x));
  morton |= uint32_t(step(tenter, t_mid.y) * step(ray.dir.y, 0.0)) << 1;
  morton |= uint32_t(step(t_mid.y, tenter) * step(0.0, ray.dir.y)) << 1;
  morton |= uint32_t(step(tenter, t_mid.z) * step(ray.dir.z, 0.0)) << 2;
  morton |= uint32_t(step(t_mid.z, tenter) * step(0.0, ray.dir.z)) << 2;
  return morton;
}

vec3 voxel_child_position_offset(uint32_t morton, float half_length) {
  half_length /= 2;
  return vec3(
    (morton & 1) > 0 ? half_length : -half_length,
    (morton & 2) > 0 ? half_length : -half_length,
    (morton & 4) > 0 ? half_length : -half_length
  );
}

u32vec3 voxel_intersection_exit_axes(in RayAABBIntersection intersection) {
  return u32vec3(
    step(intersection.tmax.x, intersection.texit),
    step(intersection.tmax.y, intersection.texit),
    step(intersection.tmax.z, intersection.texit)
  );
}

bool voxel_ray_exits_parent(vec3 ray_dir, uint32_t local_morton, uint32_t flipped_local_morton) {
  if((flipped_local_morton & 1) < (local_morton & 1) && ray_dir.x > 0) return true;
  if((flipped_local_morton & 1) > (local_morton & 1) && ray_dir.x < 0) return true;

  if((flipped_local_morton & 2) < (local_morton & 2) && ray_dir.y > 0) return true;
  if((flipped_local_morton & 2) > (local_morton & 2) && ray_dir.y < 0) return true;

  if((flipped_local_morton & 4) < (local_morton & 4) && ray_dir.z > 0) return true;
  if((flipped_local_morton & 4) > (local_morton & 4) && ray_dir.z < 0) return true;

  return false;
}
