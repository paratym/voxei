const float VOXEL_WORLD_LENGTH = 1.0;

const uint32_t BRICK_LENGTH = 8;
const uint32_t BRICK_AREA = BRICK_LENGTH * BRICK_LENGTH;
const uint32_t BRICK_VOLUME = BRICK_AREA * BRICK_LENGTH;
const uint32_t BRICK_MORTON_LENGTH = 9;
const float BRICK_WORLD_LENGTH = BRICK_LENGTH * VOXEL_WORLD_LENGTH;

const uint32_t CHUNK_LENGTH = 8;
const uint32_t CHUNK_AREA = CHUNK_LENGTH * CHUNK_LENGTH;
const uint32_t CHUNK_VOLUME = CHUNK_AREA * CHUNK_LENGTH;
const uint32_t CHUNK_VOXEL_LENGTH = CHUNK_LENGTH * BRICK_LENGTH;
const float CHUNK_WORLD_LENGTH = CHUNK_VOXEL_LENGTH * VOXEL_WORLD_LENGTH;

const uint32_t SUPER_CHUNK_LENGTH = 4;
const uint32_t SUPER_CHUNK_AREA = SUPER_CHUNK_LENGTH * SUPER_CHUNK_LENGTH;
const uint32_t SUPER_CHUNK_VOLUME = SUPER_CHUNK_AREA * SUPER_CHUNK_LENGTH;
