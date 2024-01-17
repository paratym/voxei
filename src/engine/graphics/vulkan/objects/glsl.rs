#[derive(Default, Debug, Copy, Clone)]
pub struct GlslFloat {
    pub val: f32,
}

impl GlslFloat {
    pub fn new(val: f32) -> Self {
        Self { val }
    }
}

impl GlslType for GlslFloat {
    fn size() -> usize {
        std::mem::size_of::<f32>()
    }

    fn alignment() -> usize {
        std::mem::align_of::<f32>()
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GlslVec2f {
    pub x: f32,
    pub y: f32,
}

impl GlslVec2f {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl GlslType for GlslVec2f {
    fn size() -> usize {
        std::mem::size_of::<f32>() * 2
    }

    fn alignment() -> usize {
        std::mem::align_of::<f32>() * 2
    }
}

#[derive(Default, Debug, Copy, Clone)]
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

impl Into<GlslVec3f> for nalgebra::Vector3<f32> {
    fn into(self) -> GlslVec3f {
        GlslVec3f::new(self.x, self.y, self.z)
    }
}

impl GlslType for GlslVec3f {
    fn size() -> usize {
        std::mem::size_of::<f32>() * 3
    }

    fn alignment() -> usize {
        std::mem::align_of::<f32>() * 4
    }
}

#[derive(Default, Debug, Copy, Clone)]
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

impl GlslType for GlslVec4f {
    fn size() -> usize {
        std::mem::size_of::<f32>() * 4
    }

    fn alignment() -> usize {
        std::mem::align_of::<f32>() * 4
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GlslMat4f {
    pub arr: [f32; 16],
}

impl GlslMat4f {
    pub fn new(arr: [f32; 16]) -> Self {
        Self { arr }
    }
}

impl GlslType for GlslMat4f {
    fn size() -> usize {
        std::mem::size_of::<f32>() * 16
    }

    fn alignment() -> usize {
        std::mem::align_of::<f32>() * 4
    }
}

pub trait GlslType {
    fn size() -> usize;
    fn alignment() -> usize;
}

pub struct GlslDataBuilder {
    data: Vec<u8>,
}

impl GlslDataBuilder {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push<T: GlslType>(&mut self, val: T) {
        let size = T::size();
        let alignment = T::alignment();
        let offset = self.data.len();

        let padding = (alignment - (offset % alignment)) % alignment;
        self.data.resize(offset + padding + size, 0);
        unsafe {
            let ptr = self.data.as_mut_ptr().add(offset + padding) as *mut T;
            ptr.write(val);
        }
    }

    pub fn build(self) -> Vec<u8> {
        self.data
    }
}
