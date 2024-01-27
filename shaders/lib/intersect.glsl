vec2 ray_box_intersection(Ray ray, vec3 box_min, vec3 box_max) {
  vec3 t0 = (box_min - ray.origin) * ray.inv_dir;
  vec3 t1 = (box_max - ray.origin) * ray.inv_dir;
  vec3 tmin = min(t0, t1);
  vec3 tmax = max(t0, t1);

  vec2 traverse = max(tmin.xx, tmin.yz);
  float tenter = max(traverse.x, traverse.y);

  traverse = min(tmax.xx, tmax.yz);
  float texit = min(traverse.x, traverse.y);

  return vec2(tenter, texit);
}
