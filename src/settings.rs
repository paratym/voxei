use std::f32::consts;

use voxei_macros::Resource;

#[derive(Resource)]
pub struct Settings {
    pub camera_fov: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_fov: consts::FRAC_PI_2,
        }
    }
}
