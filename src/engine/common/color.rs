/// Base color struct in rgba format.
pub struct Color {
    pub color_space: ColorSpace,
    pub xyz: [f32; 3],
    pub alpha: f32,
}

pub enum ColorSpace {
    RGB,
}

impl Color {
    pub fn new(color_space: ColorSpace, xyz: [f32; 3], alpha: f32) -> Self {
        Self {
            color_space,
            xyz,
            alpha,
        }
    }

    pub fn new_rgb(r: f32, g: f32, b: f32, alpha: f32) -> Self {
        Self {
            color_space: ColorSpace::RGB,
            xyz: [r, g, b],
            alpha,
        }
    }

    pub fn rgba(&self) -> [f32; 4] {
        [self.xyz[0], self.xyz[1], self.xyz[2], self.alpha]
    }
}
