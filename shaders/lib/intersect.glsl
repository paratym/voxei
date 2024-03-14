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
  return a.min <= b.max && a.max >= b.min;
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

  vec3 e0_up = cross(tri_e0, up_norm);
  vec3 e1_up = cross(tri_e1, up_norm);
  vec3 e2_up = cross(tri_e2, up_norm);
  vec3 e0_right = cross(tri_e0, right_norm);
  vec3 e1_right = cross(tri_e1, right_norm);
  vec3 e2_right = cross(tri_e2, right_norm);
  vec3 e0_forward = cross(tri_e0, forward_norm);
  vec3 e1_forward = cross(tri_e1, forward_norm);
  vec3 e2_forward = cross(tri_e2, forward_norm);

  Projection tri_proj_tri_norm = Projection(
    min(dot(triangle.v0.position, tri_norm), 
      min(dot(triangle.v1.position, tri_norm), dot(triangle.v2.position, tri_norm))),
    max(dot(triangle.v0.position, tri_norm), 
      max(dot(triangle.v1.position, tri_norm), dot(triangle.v2.position, tri_norm))));
  Projection tri_proj_tri_e0_norm = Projection(
    min(dot(triangle.v0.position, tri_e0_norm), 
      min(dot(triangle.v1.position, tri_norm), dot(triangle.v2.position, tri_e0_norm))),
    max(dot(triangle.v0.position, tri_e0_norm), 
      max(dot(triangle.v1.position, tri_norm), dot(triangle.v2.position, tri_e0_norm))));
  Projection tri_proj_tri_e1_norm = Projection(
    min(dot(triangle.v0.position, tri_e1_norm), 
      min(dot(triangle.v1.position, tri_e1_norm), dot(triangle.v2.position, tri_e1_norm))),
    max(dot(triangle.v0.position, tri_e1_norm), 
      max(dot(triangle.v1.position, tri_e1_norm), dot(triangle.v2.position, tri_e1_norm))));
  Projection tri_proj_tri_e2_norm = Projection(
    min(dot(triangle.v0.position, tri_e2_norm), 
      min(dot(triangle.v1.position, tri_e2_norm), dot(triangle.v2.position, tri_e2_norm))),
    max(dot(triangle.v0.position, tri_e2_norm), 
      max(dot(triangle.v1.position, tri_e2_norm), dot(triangle.v2.position, tri_e2_norm))));
  Projection tri_proj_up_norm = Projection(
    min(dot(triangle.v0.position, up_norm), 
      min(dot(triangle.v1.position, up_norm), dot(triangle.v2.position, up_norm))),
    max(dot(triangle.v0.position, up_norm), 
      max(dot(triangle.v1.position, up_norm), dot(triangle.v2.position, up_norm))));
  Projection tri_proj_right_norm = Projection(
    min(dot(triangle.v0.position, right_norm), 
      min(dot(triangle.v1.position, right_norm), dot(triangle.v2.position, right_norm))),
    max(dot(triangle.v0.position, right_norm), 
      max(dot(triangle.v1.position, right_norm), dot(triangle.v2.position, right_norm))));
  Projection tri_proj_forward_norm = Projection(
    min(dot(triangle.v0.position, forward_norm), 
      min(dot(triangle.v1.position, forward_norm), dot(triangle.v2.position, forward_norm))),
    max(dot(triangle.v0.position, forward_norm), 
      max(dot(triangle.v1.position, forward_norm), dot(triangle.v2.position, forward_norm))));
  Projection tri_proj_e0_up = Projection(
    min(dot(triangle.v0.position, e0_up), 
      min(dot(triangle.v1.position, e0_up), dot(triangle.v2.position, e0_up))),
    max(dot(triangle.v0.position, e0_up), 
      max(dot(triangle.v1.position, e0_up), dot(triangle.v2.position, e0_up))));
  Projection tri_proj_e1_up = Projection(
    min(dot(triangle.v0.position, e1_up), 
      min(dot(triangle.v1.position, e1_up), dot(triangle.v2.position, e1_up))),
    max(dot(triangle.v0.position, e1_up), 
      max(dot(triangle.v1.position, e1_up), dot(triangle.v2.position, e1_up))));
  Projection tri_proj_e2_up = Projection(
    min(dot(triangle.v0.position, e2_up), 
      min(dot(triangle.v1.position, e2_up), dot(triangle.v2.position, e2_up))),
    max(dot(triangle.v0.position, e2_up), 
      max(dot(triangle.v1.position, e2_up), dot(triangle.v2.position, e2_up))));
  Projection tri_proj_e0_right = Projection(
    min(dot(triangle.v0.position, e0_right), 
      min(dot(triangle.v1.position, e0_right), dot(triangle.v2.position, e0_right))),
    max(dot(triangle.v0.position, e0_right), 
      max(dot(triangle.v1.position, e0_right), dot(triangle.v2.position, e0_right))));
  Projection tri_proj_e1_right = Projection(
    min(dot(triangle.v0.position, e1_right), 
      min(dot(triangle.v1.position, e1_right), dot(triangle.v2.position, e1_right))),
    max(dot(triangle.v0.position, e1_right), 
      max(dot(triangle.v1.position, e1_right), dot(triangle.v2.position, e1_right))));
  Projection tri_proj_e2_right = Projection(
    min(dot(triangle.v0.position, e2_right), 
      min(dot(triangle.v1.position, e2_right), dot(triangle.v2.position, e2_right))),
    max(dot(triangle.v0.position, e2_right), 
      max(dot(triangle.v1.position, e2_right), dot(triangle.v2.position, e2_right))));
  Projection tri_proj_e0_forward = Projection(
    min(dot(triangle.v0.position, e0_forward), 
      min(dot(triangle.v1.position, e0_forward), dot(triangle.v2.position, e0_forward))),
    max(dot(triangle.v0.position, e0_forward), 
      max(dot(triangle.v1.position, e0_forward), dot(triangle.v2.position, e0_forward))));
  Projection tri_proj_e1_forward = Projection(
    min(dot(triangle.v0.position, e1_forward), 
      min(dot(triangle.v1.position, e1_forward), dot(triangle.v2.position, e1_forward))),
    max(dot(triangle.v0.position, e1_forward), 
      max(dot(triangle.v1.position, e1_forward), dot(triangle.v2.position, e1_forward))));
  Projection tri_proj_e2_forward = Projection(
    min(dot(triangle.v0.position, e2_forward), 
      min(dot(triangle.v1.position, e2_forward), dot(triangle.v2.position, e2_forward))),
    max(dot(triangle.v0.position, e2_forward), 
      max(dot(triangle.v1.position, e2_forward), dot(triangle.v2.position, e2_forward))));

  Projection aabb_project_tri_norm = Projection(
    min(dot(aabb.min, tri_norm), dot(aabb.max, tri_norm)),
    max(dot(aabb.min, tri_norm), dot(aabb.max, tri_norm)));
  Projection aabb_project_tri_e0_norm = Projection(
    min(dot(aabb.min, tri_e0_norm), dot(aabb.max, tri_e0_norm)),
    max(dot(aabb.min, tri_e0_norm), dot(aabb.max, tri_e0_norm)));
  Projection aabb_project_tri_e1_norm = Projection(
    min(dot(aabb.min, tri_e1_norm), dot(aabb.max, tri_e1_norm)),
    max(dot(aabb.min, tri_e1_norm), dot(aabb.max, tri_e1_norm)));
  Projection aabb_project_tri_e2_norm = Projection(
    min(dot(aabb.min, tri_e2_norm), dot(aabb.max, tri_e2_norm)),
    max(dot(aabb.min, tri_e2_norm), dot(aabb.max, tri_e2_norm)));
  Projection aabb_project_up_norm = Projection(
    min(dot(aabb.min, up_norm), dot(aabb.max, up_norm)),
    max(dot(aabb.min, up_norm), dot(aabb.max, up_norm)));
  Projection aabb_project_right_norm = Projection(
    min(dot(aabb.min, right_norm), dot(aabb.max, right_norm)),
    max(dot(aabb.min, right_norm), dot(aabb.max, right_norm)));
  Projection aabb_project_forward_norm = Projection(
    min(dot(aabb.min, forward_norm), dot(aabb.max, forward_norm)),
    max(dot(aabb.min, forward_norm), dot(aabb.max, forward_norm)));
  Projection aabb_project_e0_up = Projection(
    min(dot(aabb.min, e0_up), dot(aabb.max, e0_up)),
    max(dot(aabb.min, e0_up), dot(aabb.max, e0_up)));
  Projection aabb_project_e1_up = Projection(
    min(dot(aabb.min, e1_up), dot(aabb.max, e1_up)),
    max(dot(aabb.min, e1_up), dot(aabb.max, e1_up)));
  Projection aabb_project_e2_up = Projection(
    min(dot(aabb.min, e2_up), dot(aabb.max, e2_up)),
    max(dot(aabb.min, e2_up), dot(aabb.max, e2_up)));
  Projection aabb_project_e0_right = Projection(
    min(dot(aabb.min, e0_right), dot(aabb.max, e0_right)),
    max(dot(aabb.min, e0_right), dot(aabb.max, e0_right)));
  Projection aabb_project_e1_right = Projection(
    min(dot(aabb.min, e1_right), dot(aabb.max, e1_right)),
    max(dot(aabb.min, e1_right), dot(aabb.max, e1_right)));
  Projection aabb_project_e2_right = Projection(
    min(dot(aabb.min, e2_right), dot(aabb.max, e2_right)),
    max(dot(aabb.min, e2_right), dot(aabb.max, e2_right)));
  Projection aabb_project_e0_forward = Projection(
    min(dot(aabb.min, e0_forward), dot(aabb.max, e0_forward)),
    max(dot(aabb.min, e0_forward), dot(aabb.max, e0_forward)));
  Projection aabb_project_e1_forward = Projection(
    min(dot(aabb.min, e1_forward), dot(aabb.max, e1_forward)),
    max(dot(aabb.min, e1_forward), dot(aabb.max, e1_forward)));
  Projection aabb_project_e2_forward = Projection(
    min(dot(aabb.min, e2_forward), dot(aabb.max, e2_forward)),
    max(dot(aabb.min, e2_forward), dot(aabb.max, e2_forward)));

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
  if(!projection_overlap(tri_proj_e0_up, aabb_project_e0_up)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e1_up, aabb_project_e1_up)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e2_up, aabb_project_e2_up)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e0_right, aabb_project_e0_right)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e1_right, aabb_project_e1_right)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e2_right, aabb_project_e2_right)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e0_forward, aabb_project_e0_forward)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e1_forward, aabb_project_e1_forward)) {
    return false;
  }
  if(!projection_overlap(tri_proj_e2_forward, aabb_project_e2_forward)) {
    return false;
  }


  return true;
}
