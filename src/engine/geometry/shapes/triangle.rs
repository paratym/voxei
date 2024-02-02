use nalgebra::Vector3;

use super::{Face, Shape, Vertex};

pub struct Triangle {
    pub v1: Vector3<f32>,
    pub v2: Vector3<f32>,
    pub v3: Vector3<f32>,
}

impl Triangle {
    pub fn new(v1: Vector3<f32>, v2: Vector3<f32>, v3: Vector3<f32>) -> Self {
        Self { v1, v2, v3 }
    }
}

impl Shape for Triangle {
    fn collect_vertices(&self) -> Vec<Vertex> {
        vec![self.v1, self.v2, self.v3]
    }

    fn collect_faces(&self) -> Vec<Face> {
        vec![Face::new(vec![self.v1, self.v2, self.v3])]
    }
}
