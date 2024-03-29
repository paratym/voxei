use std::f32::consts;

use voxei_macros::Resource;

#[derive(Resource)]
pub struct Settings {
    pub camera_fov: f32,
    pub mouse_sensitivity: f32,

    // Radius of the render distance.
    pub chunk_render_distance: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_fov: consts::FRAC_PI_2,
            mouse_sensitivity: 0.5,
            chunk_render_distance: 4,
        }
    }
}
