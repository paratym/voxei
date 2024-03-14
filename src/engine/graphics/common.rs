#[derive(Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: (f32, f32, f32),
    _padding: f32,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: (x, y, z),
            _padding: 0.0,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct GTriangle {
    pub vertices: (Vertex, Vertex, Vertex, Vertex),
}
