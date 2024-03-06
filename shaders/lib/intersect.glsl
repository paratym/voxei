vec2 ray_aabb_intersection(Ray ray, AABB aabb) {
  vec3 t0 = (aabb.min - ray.origin) * ray.inv_dir;
  vec3 t1 = (aabb.max - ray.origin) * ray.inv_dir;
  vec3 tmin = min(t0, t1);
  vec3 tmax = max(t0, t1);

  vec2 traverse = max(tmin.xx, tmin.yz);
  float tenter = max(traverse.x, traverse.y);

  traverse = min(tmax.xx, tmax.yz);
  float texit = min(traverse.x, traverse.y);

  return vec2(tenter, texit);
}

vec2 ray_aabb_intersection_extra(Ray ray, AABB aabb, out vec3 tmin, out vec3 tmax) {
  vec3 t0 = (aabb.min - ray.origin) * ray.inv_dir;
  vec3 t1 = (aabb.max - ray.origin) * ray.inv_dir;
  tmin = min(t1, t0);
  tmax = max(t1, t0);

  vec2 traverse = max(tmin.xx, tmin.yz);
  float tenter = max(traverse.x, traverse.y);

  traverse = min(tmax.xx, tmax.yz);
  float texit = min(traverse.x, traverse.y);

  return vec2(tenter, texit);
}

