layout (local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

#include "lib/types.glsl"
#include "lib/constants.glsl"
#include "lib/intersect.glsl"

DECL_PUSH_CONSTANTS {
  ResourceId voxel_buffer;
  ResourceId svo_buffer;
  uint32_t head;
  uint32_t size;
} push_constants;

DECL_BUFFER(16) SVOList {
  uint node_length;
  SVONode nodes[];
};

DECL_BUFFER(4) VoxelList {
  int data[];
};

i32vec3 morton_to_position(uint32_t index, int half_length) {
  i32vec3 position = i32vec3(0);
  int curr_length = 1;
  while(true) {
    i32vec3 sign = i32vec3(
      ((index & 1) > 0) ? curr_length - 1 : -curr_length,
      ((index & 4) > 0) ? curr_length - 1 : -curr_length,
      ((index & 2) > 0) ? curr_length - 1 : -curr_length
    );
    position += sign;

    if(curr_length == half_length) {
      return position;
    }

    index = index >> 3;
    curr_length <<= 1;
  }
  return position;
}

void add_voxel(uint morton, bool filled, inout SVOList svo_list) {
  svo_list.nodes[morton + 1] = SVONode(filled ? 1 : 0, 0, uint[2](0xFFFFFFFF, 0xFFFFFFFF));
}

void main() {
  VoxelList voxel_list = get_buffer(push_constants.voxel_buffer, VoxelList);
  SVOList svo_list = get_buffer(push_constants.svo_buffer, SVOList);

  uint32_t index = gl_GlobalInvocationID.x;
  int half_length = int(log2(push_constants.size) / 3);

  if(index >= push_constants.size) {
    return;
  }
  if(index == 0) {
    // Set the null node.
    svo_list.nodes[0] = SVONode(0, 0, uint[2](0xFFFFFFFF, 0xFFFFFFFF));
  }

  uint head = push_constants.head;
  if(head == 1) {
    add_voxel(index, voxel_list.data[index] == 1, svo_list);
  } else {
    uint previous_size = push_constants.size * 8;
    uint previous_head = push_constants.head - previous_size;

    uint child_offset = previous_head + index * 8;
    uint children[] = uint[2](0xFFFFFFFF, 0xFFFFFFFF);
    for(int i = 0; i < 8; i++) {
      uint ptr = child_offset + i;

      SVONode node = svo_list.nodes[ptr];
      if(!(node.data_index == 0 && node.child_index == 0)) {
        uint h = i / 4;
        uint byte_index = i % 4;
        uint and = ~(0xFF << (byte_index * 8));

        children[h] = (children[h] & and) | ((i) << (byte_index * 8));
      }
    }
    svo_list.nodes[head + index] = SVONode(0, child_offset, children);

    if(push_constants.size == 1) {
      svo_list.node_length = head + 1;
    }
  }
}
