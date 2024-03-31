pub const APP_NAME: &str = "voxei";
pub const WINDOW_TITLE: &str = "Voxei";

pub const VULKAN_ENGINE_NAME: &str = APP_NAME;
pub const VULKAN_APP_NAME: &str = APP_NAME;
pub const VULKAN_DEFAULT_QUEUE: &str = "vulkan_default";

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

// Length in voxels.
pub const BRICK_LENGTH: u64 = 8;
pub const BRICK_AREA: u64 = BRICK_LENGTH * BRICK_LENGTH;
pub const BRICK_VOLUME: u64 = BRICK_LENGTH * BRICK_LENGTH * BRICK_LENGTH;

// Length in bricks.
pub const CHUNK_LENGTH: u64 = 8;

// Max # of bricks to load onto the gpu per frame.
pub const MAX_BRICK_LOAD: usize = 64;

// Max # of brick the gpu can request per frame.
pub const MAX_BRICK_REQUEST: usize = 64;

// Max # of bricks the gpu can have loaded, todo - make dynamic based on available vram.
pub const MAX_BRICK_DATA: usize = 1024;
