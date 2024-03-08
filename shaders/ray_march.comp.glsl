layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#include "lib/types.glsl"
#include "lib/intersect.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId backbuffer_id;
  ResourceId camera_id;
  ResourceId svo_id;
  uint32_t subdivisions;
} push_constants;

DECL_BUFFER(16) Camera {
  mat4 view;
  u32vec2 resolution;
  float aspect;
  float fov;
};

DECL_BUFFER(16) SVOList {
  uint node_length;
  SVONode nodes[];
};

float get_half_length(uint depth) {
  return 0.5 * exp2(float(push_constants.subdivisions - depth));
}

// eg. ray_box_intersection(ray, vec3(0.0), 0)
// with subdivisions of 3 would be calculating the bounds
// of a min (-4, -4, -4) and max (4, 4, 4) box
vec2 ray_voxel_intersection(Ray ray, vec3 pos, uint depth, out vec3 tmin, out vec3 tmax) {
  float half_length = get_half_length(depth);
  return ray_aabb_intersection_extra(ray, AABB(pos - half_length, pos + half_length), tmin, tmax);
}

// Returns the 3 digit morton code for the child voxel local position
// ray - the ray to test the direction of
// tmid - the t values of the voxels axes planes intersected by the ray
// tc - the t values of the entry and exit of the parent voxel
uint get_child_local_morton(Ray ray, vec3 tmid, vec2 tc) {
  uint i = 0;
  // X
  if (tmid.x <= tc.x) {
    if (ray.dir.x > 0.0) {
      i |= 1;
    }
  } else {
    if (ray.dir.x < 0.0) {
      i |= 1;
    }
  }

  // Y 
  if (tmid.y <= tc.x) {
    if (ray.dir.y > 0.0) {
      i |= 2;
    }
  } else {
    if (ray.dir.y < 0.0) {
      i |= 2;
    }
  }

  // Z
  if (tmid.z <= tc.x) {
    if (ray.dir.z > 0.0) {
      i |= 4;
    }
  } else {
    if (ray.dir.z < 0.0) {
      i |= 4;
    }
  }
  return i;

}

vec3 get_child_position(vec3 parent_pos, uint child_local_morton, uint child_depth) {
  float half_length = get_half_length(child_depth);
  vec3 child_mask = vec3(
    (child_local_morton & 1) > 0 ? 1.0 : -1.0,
    (child_local_morton & 2) > 0 ? 1.0 : -1.0,
    (child_local_morton & 4) > 0 ? 1.0 : -1.0
  );
  return parent_pos + (child_mask * half_length * 0.5);
}

uint get_child_node_index(uint parent_node_index, uint child_local_morton, inout SVOList svo_list) {
  SVONode parent_node = svo_list.nodes[parent_node_index];
  uint child_node_index = parent_node.child_index;
  if(child_node_index == 0) {
    return 1;
  }
  uint grouped_child_offset = parent_node.child_offsets[child_local_morton / 4];
  // Since the offset is stored as 2xu32, we need to interpret it as 8xu8 
  uint child_offset = (grouped_child_offset >> ((child_local_morton % 4) * 8)) & 0xFF;
  if(child_offset == 255) {
    return 0;
  }
  return child_node_index + child_offset;
}

struct TraceOutput {
  bool hit;
  vec3 pos;
  vec3 color;
};

struct StackItem {
  uint node_index;
  vec3 pos;
};

TraceOutput trace(Ray ray) {
  SVOList svo_list = get_buffer(push_constants.svo_id, SVOList);
  float side_length = 1 << push_constants.subdivisions;
  float half_length = side_length / 2;
  AABB bounds = AABB(vec3(-half_length), vec3(half_length));

  vec2 root_t_corners = ray_aabb_intersection(ray, bounds);
  // If the ray starts inside the svo, have the tenter be 0 since this seems to work
  root_t_corners.x = max(root_t_corners.x, 0.0);
  bool hit = root_t_corners.y >= root_t_corners.x;
  vec3 color = vec3(0.01, 0.05, 0.05);
  if(!hit) {
    return TraceOutput(false, vec3(0.0), color);
  }
  color = vec3(0,1,0);

  uint parent_node_index = svo_list.node_length - 1;
  SVONode parent_node = svo_list.nodes[parent_node_index];
  // The center position of the root node.
  vec3 root_pos = bounds.min + ((bounds.max - bounds.min) * 0.5);
  vec3 root_t_mid = (root_pos - ray.origin) * ray.inv_dir;
  uint first_child_local_morton = get_child_local_morton(ray, root_t_mid, root_t_corners);
  vec3 first_child_pos = get_child_position(root_pos, first_child_local_morton, 0);

  uint depth = 1u;
  vec3 pos = first_child_pos;
  uint morton = first_child_local_morton;
  // Holds parent indices
  StackItem stack[10];
  uint stack_ptr = 0;

  bool dont_push = false;
  float h = root_t_corners.y;
  for(uint i = 0; i < 128; i++) {
    // Calculate the intersection of the ray with the current voxel
    vec3 tmin, tmax;
    vec2 tc = ray_voxel_intersection(ray, pos, depth, tmin, tmax);
    tc.x = max(tc.x, 0.0);
    bool hit = tc.y >= tc.x;
    uint local_morton = morton & 7;
    if(hit) {
      uint node_index = get_child_node_index(parent_node_index, local_morton, svo_list);
      if(node_index != 0 && !dont_push) {
        SVONode child = svo_list.nodes[node_index];
        if (child.data_index != 0) {
          const vec3 LIGHT_POS = vec3(2.5, 3.5, 3.5);
          color = vec3(node_index/74.0);
          break;
        }

        stack[depth] = StackItem(parent_node_index, pos);

        parent_node_index = node_index;
        vec3 t_mid = (pos - ray.origin) * ray.inv_dir;
        uint child_morton = get_child_local_morton(ray, t_mid, tc);
        pos = get_child_position(pos, child_morton, depth);
        morton = (morton << 3) | child_morton;
        depth++;
        continue;
      }
    } else {
    }

    // We no longer have a valid child node so we are in empty space and need to advance
    // our ray to the next voxel, we do this by finding which axis the ray exits the current voxel on,
    // calculate the flipped bit in the local morton code, and compare that against the ray direction
    // to determine if we are no longer in the same parent. 

    // Example: a ray going from -x to +x and advancing from 01 to 00 will have a flipped bit in the x axis, 
    // since the ray is increasing and the code decreased, we know we are no longer in the same parent.

    // Check which axis we exit the voxel on
    bvec3 exit_axis = bvec3(
      tc.y >= tmax.x,
      tc.y >= tmax.y,
      tc.y >= tmax.z
    );
    uint exit_bits = uint(exit_axis.x) + (uint(exit_axis.y) << 1) + (uint(exit_axis.z) << 2);
    uint flipped_local_morton = local_morton ^ exit_bits;

    morton = ((morton >> 3) << 3) | flipped_local_morton;

    // xor the exit axis with the ones to get the flipped bits
    vec3 ones = vec3(
      (local_morton & 1) > 0 ? 1 : 0,
      (local_morton & 2) > 0 ? 1 : 0,
      (local_morton & 4) > 0 ? 1 : 0
    );
    vec3 new_ones = vec3(
      (flipped_local_morton & 1) > 0 ? 1 : 0,
      (flipped_local_morton & 2) > 0 ? 1 : 0,
      (flipped_local_morton & 4) > 0 ? 1 : 0
    );
    pos += sign(ray.dir) * vec3(exit_axis) * get_half_length(depth) * 2; // correct for our xyz being diff

    // Check if the ray is increasing or decreasing corresponding to the flipped bits
    bool against_ray_dir = false;
    if (new_ones.x < ones.x && ray.dir.x > 0) {
      against_ray_dir = true;
    }
    if (new_ones.y < ones.y && ray.dir.y > 0) {
      against_ray_dir = true;
    }
    if (new_ones.z < ones.z && ray.dir.z > 0) {
      against_ray_dir = true;
    }
    if (new_ones.x > ones.x && ray.dir.x < 0) {
      against_ray_dir = true;
    }
    if (new_ones.y > ones.y && ray.dir.y < 0) {
      against_ray_dir = true;
    }
    if (new_ones.z > ones.z && ray.dir.z < 0) {
      against_ray_dir = true;
    }
    // color = vec3(exit_axis.x ? 1.0 : 0.0, exit_axis.y ? 1.0 : 0.0, exit_axis.z ? 1.0 : 0.0);

    if(against_ray_dir) {
      depth--;
      if(depth == 0) {
        break;
      }

      // If ray exits models bounding box
      vec3 min = bounds.min;
      vec3 max = bounds.max;
      if(pos.x < min.x || pos.x > max.x ||
         pos.y < min.y || pos.y > max.y ||
         pos.z < min.z || pos.z > max.z) {
        break;
      }

      // color = vec3(depth / 2.0, 0.0, 0.0);

      StackItem item = stack[depth];
      parent_node_index = item.node_index;
      pos = item.pos;
      morton = morton >> 3;

      // Makes it so the bits are flipped the next iteration so we don't go back into the same parent
      dont_push = true;
      h = 10000;
    } else {
      dont_push = false;
    }
  }

  return TraceOutput(false, vec3(0.0), color);
}
void main() {
  Camera camera = get_buffer(push_constants.camera_id, Camera);

  vec2 coord = gl_GlobalInvocationID.xy;
  if(coord.x > camera.resolution.x || coord.y > camera.resolution.y) {
    return;
  }

  vec2 ndc = coord / camera.resolution;
  vec2 uv = vec2(ndc.x * 2.0 - 1.0, 1 - 2 * ndc.y);
  vec2 scaled_uv = vec2(uv.x * camera.aspect, uv.y) * tan(camera.fov / 2.0);

  vec3 ro = vec3(vec4(0.0, 0.0, 0.0, 1.0) * camera.view);
  vec3 rd = normalize(vec3(scaled_uv, 1.0)) * mat3(camera.view);

  Ray ray = Ray(ro, rd, 1.0 / rd);

  TraceOutput trace_out = trace(Ray(ro, rd, 1.0 / rd));

  imageStore(get_storage_image(push_constants.backbuffer_id), ivec2(coord), vec4(trace_out.color, 1.0));
}
