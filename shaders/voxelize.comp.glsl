#version 450

#extension GL_EXT_shader_explicit_arithmetic_types_int32 : enable
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : enable
#extension GL_EXT_buffer_reference : enable
#extension GL_EXT_debug_printf : enable


layout (set = 0, binding = 0) buffer BufferAddresses {
  uint64_t addresses[];
} u_addresses;
layout (set = 0, binding = 1, rgba8) uniform image2D u_images[100];

struct ResourceId {
  uint32_t index;
};

#define DECL_PUSH_CONSTANTS layout(push_constant) uniform PushConstants
#define DECL_BUFFER(alignment) layout(std430, buffer_reference, buffer_reference_align = alignment) buffer

#define get_buffer(id, type) type(u_addresses.addresses[id.index]);
#define get_storage_image(id) u_images[id.index]

// Shader Specific Code
layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

DECL_PUSH_CONSTANTS {
  ResourceId backbuffer_id;
  ResourceId camera_id;
} push_constants;

DECL_BUFFER(16) Camera {
  u32vec2 resolution;
  float aspect;
};
