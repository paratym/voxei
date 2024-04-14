use std::f32::consts;

use voxei_macros::Resource;

use crate::engine::voxel::vox_world::ChunkRadius;

#[derive(Resource)]
pub struct Settings {
    /// The field of view in degrees of the camera.
    pub camera_fov: f32,

    /// The mouse sensitivity of pixels per degree of rotation.
    pub mouse_sensitivity: f32,

    /// The radius of the max # of chunks to try and render.
    pub chunk_render_distance: ChunkRadius,

    /// The radius of the # of chunk that should try and stay dynamically loaded, this means we
    /// will store them in the dyn world brick array even if a ray hasn't requested it.
    pub chunk_dyn_loaded_distance: ChunkRadius,

    /// The radius of the max # of chunks that should try being cached in the
    /// static world with the minimum being the chunk_render_distance.
    /// Any chunk that has not yet been generated will not be loaded.
    pub chunk_loaded_distance: ChunkRadius,

    /// The max radius of chunks that should actively generate around the player, even if a ray
    /// has not requested it.
    pub chunk_generation_distance: ChunkRadius,

    /// The max # of bricks that can be stored in the brick data buffer.
    pub brick_data_max_size: u32,

    /// The max # of bricks that can requested per frame on the gpu.
    pub brick_request_max_size: u32,

    /// The max # of bricks that can be uploaded to the gpu per frame.
    pub brick_load_max_size: u32,

    /// The real world side length of 1x1x1 voxel.
    pub voxel_unit_length: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_fov: consts::FRAC_PI_2,
            mouse_sensitivity: 0.05,

            chunk_render_distance: ChunkRadius::new(32),
            chunk_dyn_loaded_distance: ChunkRadius::new(5),
            chunk_loaded_distance: ChunkRadius::new(32),
            chunk_generation_distance: ChunkRadius::new(4),

            brick_data_max_size: 100000,
            brick_request_max_size: 64,
            brick_load_max_size: 256,

            voxel_unit_length: 0.5,
        }
    }
}
