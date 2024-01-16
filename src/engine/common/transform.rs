use nalgebra::{Isometry3, Vector3};

pub struct Transform {
    pub isometry: Isometry3<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            isometry: Isometry3::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn to_matrix(&self) -> nalgebra::Matrix4<f32> {
        let mut matrix = self.isometry.to_homogeneous();
        matrix[(0, 0)] *= self.scale.x;
        matrix[(1, 1)] *= self.scale.y;
        matrix[(2, 2)] *= self.scale.z;
        matrix
    }
}
