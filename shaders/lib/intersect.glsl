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

bool aabb_aabb_intersection(AABB a, AABB b) {
  return (a.min.x <= b.max.x && a.max.x >= b.min.x) &&
         (a.min.y <= b.max.y && a.max.y >= b.min.y) &&
         (a.min.z <= b.max.z && a.max.z >= b.min.z);
}

struct Projection {
  float min;
  float max;
};

bool projection_overlap(Projection a, Projection b) {
  // A will the tri in this case, so if the min is (0, 0, 0)
  // we want to align voxels with their min starting there also
  // and not their max, so we don't double generate voxels
  return a.min < b.max && a.max > b.min;
}

Projection project_tri_to_axes(Triangle triangle, vec3 axis) {
  float v0 = dot(triangle.v0.position, axis);
  float v1 = dot(triangle.v1.position, axis);
  float v2 = dot(triangle.v2.position, axis);
  return Projection(min(v0, min(v1, v2)), max(v0, max(v1, v2)));
}

Projection project_aabb_to_axes(AABB aabb, vec3 axis) {
  vec3 length = aabb.max - aabb.min;
  float v0 = dot(aabb.min, axis);
  float v1 = dot(aabb.min + vec3(length.x,0,0), axis);
  float v2 = dot(aabb.min + vec3(0,length.y,0), axis);
  float v3 = dot(aabb.min + vec3(0,0,length.z), axis);
  float v4 = dot(aabb.min + vec3(length.x,length.y,0), axis);
  float v5 = dot(aabb.min + vec3(length.x,0,length.z), axis);
  float v6 = dot(aabb.min + vec3(0,length.y,length.z), axis);
  float v7 = dot(aabb.max, axis);

  float min = min(v0, min(v1, min(v2, min(v3, min(v4, min(v5, min(v6, v7)))))));
  float max = max(v0, max(v1, max(v2, max(v3, max(v4, max(v5, max(v6, v7)))))));

  return Projection(min, max);
}

bool triangle_aabb_intersection(Triangle triangle, AABB aabb) {
  vec3 tri_e0 = normalize(triangle.v1.position - triangle.v0.position);
  vec3 tri_e1 = normalize(triangle.v2.position - triangle.v0.position);
  vec3 tri_e2 = normalize(triangle.v1.position - triangle.v2.position);

  vec3 tri_norm = cross(tri_e1, tri_e0);
  vec3 tri_e0_norm = cross(tri_e0, tri_norm);
  vec3 tri_e1_norm = cross(tri_e1, tri_norm);
  vec3 tri_e2_norm = cross(tri_e2, tri_norm);

  vec3 up_norm = vec3(0, 1, 0);
  vec3 right_norm = vec3(1, 0, 0);
  vec3 forward_norm = vec3(0, 0, 1);

  Projection tri_proj_tri_norm = project_tri_to_axes(triangle, tri_norm);
  Projection tri_proj_tri_e0_norm = project_tri_to_axes(triangle, tri_e0_norm);
  Projection tri_proj_tri_e1_norm = project_tri_to_axes(triangle, tri_e1_norm);
  Projection tri_proj_tri_e2_norm = project_tri_to_axes(triangle, tri_e2_norm);
  Projection tri_proj_up_norm = project_tri_to_axes(triangle, up_norm);
  Projection tri_proj_right_norm = project_tri_to_axes(triangle, right_norm);
  Projection tri_proj_forward_norm = project_tri_to_axes(triangle, forward_norm);

  Projection aabb_project_tri_norm = project_aabb_to_axes(aabb, tri_norm);
  Projection aabb_project_tri_e0_norm = project_aabb_to_axes(aabb, tri_e0_norm);
  Projection aabb_project_tri_e1_norm = project_aabb_to_axes(aabb, tri_e1_norm);
  Projection aabb_project_tri_e2_norm = project_aabb_to_axes(aabb, tri_e2_norm);
  Projection aabb_project_up_norm = project_aabb_to_axes(aabb, up_norm);
  Projection aabb_project_right_norm = project_aabb_to_axes(aabb, right_norm);
  Projection aabb_project_forward_norm = project_aabb_to_axes(aabb, forward_norm);

  if(!projection_overlap(tri_proj_tri_norm, aabb_project_tri_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_tri_e0_norm, aabb_project_tri_e0_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_tri_e1_norm, aabb_project_tri_e1_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_tri_e2_norm, aabb_project_tri_e2_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_up_norm, aabb_project_up_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_right_norm, aabb_project_right_norm)) {
    return false;
  }
  if(!projection_overlap(tri_proj_forward_norm, aabb_project_forward_norm)) {
    return false;
  }

  return true;
}
