layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#include "lib/constants.glsl"
#include "lib/types.glsl"
#include "lib/intersect.glsl"
#include "lib/utils.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId backbuffer_id;
  ResourceId camera_id;
  ResourceId chunk_tree_id;
  ResourceId chunk_data_lut_id;
  ResourceId voxel_data_id;
} push_constants;

struct TraceWorldOut {
  VoxelData voxel;
  bool hit;
};

TraceWorldOut trace_world_out_miss() {
  return TraceWorldOut(voxel_data_empty(), false);
}

TraceWorldOut trace_world_out_hit(vec3 color) {
  return TraceWorldOut(VoxelData(color), true);
}

struct StackItem {
  uint32_t node_index;
  bool is_chunk_octant;
  vec3 pos;
};

const uint32_t MARCH_ITERATIONS = 2048;
const uint32_t MAX_DEPTH = 20;

TraceWorldOut trace_vox_world(Ray ray) {
  ChunkOctree chunk_tree = get_buffer(push_constants.chunk_tree_id, ChunkOctree);
  ChunkDataLUT chunk_data_lut = get_buffer(push_constants.chunk_data_lut_id, ChunkDataLUT);
  VoxelDataList voxel_data_list = get_buffer(push_constants.voxel_data_id, VoxelDataList);

  vec3 root_pos = vec3(0.0);
  float curr_half_length = (chunk_tree.side_length * CHUNK_UNIT_LENGTH) / 2; 
  RayAABBIntersection root_isect = ray_voxel_intersection(ray, root_pos, curr_half_length);

  uint32_t curr_morton = voxel_ray_child_morton(ray, root_pos, root_isect.tenter);
  vec3 curr_pos = root_pos + voxel_child_position_offset(curr_morton, curr_half_length);
  curr_half_length /= 2;
  uint32_t curr_node = 0;
  uint32_t curr_chunk_data = NULL_NODE;

  StackItem stack[MAX_DEPTH];
  uint32_t curr_stack_ptr = 0;
  bool should_push = true;

  for(uint32_t i = 0; i < 100; i++) {
    RayAABBIntersection voxel_isect = ray_voxel_intersection(ray, curr_pos, curr_half_length);
    if(!voxel_isect.hit) {
      return trace_world_out_miss();
    }

    // Calculate the node index of our intersected child voxel
    uint32_t local_morton = curr_morton & 7;
    uint32_t node_index = NULL_NODE;
    if(curr_chunk_data == NULL_NODE) {
      ChunkOctreeNode node_data = chunk_tree.nodes[curr_node];
      if(node_data.chunk_data != NULL_NODE) {
        curr_chunk_data = node_data.chunk_data;
        VoxelOctree chunk_data = get_buffer(chunk_data_lut.chunks[curr_chunk_data], VoxelOctree);
        VoxelOctreeNode vox_node_data = chunk_data.nodes[0];
        curr_node = 0;
        node_index = vox_node_data.children[local_morton];
      } else {
        node_index = node_data.children[local_morton];
      }
    } else {
      VoxelOctree chunk_data = get_buffer(chunk_data_lut.chunks[curr_chunk_data], VoxelOctree);
      VoxelOctreeNode node_data = chunk_data.nodes[curr_node];
      if(node_data.voxel_data != NULL_NODE) {
        return trace_world_out_hit(voxel_data_list.data[node_data.voxel_data].color);
      }

      node_index = node_data.children[local_morton];
    }

    if(node_index != NULL_NODE && should_push) {
      stack[curr_stack_ptr] = StackItem(curr_node, curr_chunk_data == NULL_NODE, curr_pos);
      curr_stack_ptr++;

      uint32_t child_morton  = voxel_ray_child_morton(ray, curr_pos, voxel_isect.tenter);
      curr_pos += voxel_child_position_offset(child_morton, curr_half_length);
      curr_half_length /= 2;
      curr_morton = (curr_morton << 3) | child_morton;
      curr_node = node_index;
    } else {
      u32vec3 exit_axes = voxel_intersection_exit_axes(voxel_isect);
      uint32_t exit_morton = exit_axes.x | (exit_axes.y << 1) | (exit_axes.z << 2);
      uint32_t flipped_local_morton = local_morton ^ exit_morton;
      
      bool exits_parent = voxel_ray_exits_parent(ray.dir, local_morton, flipped_local_morton);

      curr_pos += sign(ray.dir) * exit_axes * curr_half_length * 2;
      curr_morton = curr_morton ^ exit_morton;

      should_push = true;
      if(exits_parent) {
        should_push = false;

        if(curr_stack_ptr == 0) {
          return trace_world_out_hit(vec3(exit_axes));
          return trace_world_out_hit(vec3(0.2, 0.2, 1.2));
          break;
        }

        curr_stack_ptr--;
        StackItem item = stack[curr_stack_ptr];
        curr_node = item.node_index;
        curr_chunk_data = item.is_chunk_octant ? curr_chunk_data : NULL_NODE;
        curr_pos = item.pos;
        curr_morton = curr_morton >> 3;
        curr_half_length *= 2;
      }
    }
  }
  return trace_world_out_hit(vec3(0.2, 0.2, 0.2));
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

  vec3 ro = vec3(vec4(0.0, 0.0, 0.0, 1.0) * camera.transform);
  vec3 rd = normalize(vec3(scaled_uv, 1.0)) * mat3(camera.transform);
  Ray ray = Ray(ro, rd, 1.0 / rd);

  TraceWorldOut trace_world_out = trace_vox_world(ray);

  vec3 color = vec3(0.0);
  if(trace_world_out.hit) {
    color = trace_world_out.voxel.color;
  }

  imageStore(get_storage_image(push_constants.backbuffer_id), ivec2(coord), vec4(color, 1.0));
}
