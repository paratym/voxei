#[repr(align(8))]
pub struct GlslVec2f {
    pub x: f32,
    pub y: f32,
}

impl GlslVec2f {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[repr(align(16))]
pub struct GlslVec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GlslVec3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[repr(align(16))]
pub struct GlslVec4f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl GlslVec4f {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

#[repr(align(64))]
pub struct GlslMat4f {
    pub arr: [f32; 16],
}

impl GlslMat4f {
    pub fn new(arr: [f32; 16]) -> Self {
        Self { arr }
    }
}

impl Default for GlslMat4f {
    fn default() -> Self {
        Self { arr: [0.0; 16] }
    }
}
