use nalgebra::Vector3;

use crate::engine::geometry::shapes::Face;

use super::Shape;

#[derive(Clone, Copy)]
pub struct AABB {
    center: Vector3<f32>,
    half_extents: Vector3<f32>,
}

impl AABB {
    pub fn new_center_half_extent(center: Vector3<f32>, half_extents: Vector3<f32>) -> Self {
        Self {
            center,
            half_extents,
        }
    }

    pub fn new_min_max(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        let half_extents = (max - min) / 2.0;
        let center = min + half_extents;
        Self {
            center,
            half_extents,
        }
    }

    pub fn into_cube(&self) -> Self {
        let half_max_length = self.half_extents.max();

        Self::new_center_half_extent(
            self.center(),
            Vector3::new(half_max_length, half_max_length, half_max_length),
        )
    }

    pub fn center(&self) -> Vector3<f32> {
        self.center
    }

    pub fn half_extents(&self) -> Vector3<f32> {
        self.half_extents
    }

    pub fn min(&self) -> Vector3<f32> {
        self.center - self.half_extents
    }

    pub fn max(&self) -> Vector3<f32> {
        self.center + self.half_extents
    }
}

impl Shape for AABB {
    fn collect_vertices(&self) -> Vec<super::Vertex> {
        let min = self.min();
        let max = self.max();
        vec![
            Vector3::new(min.x, min.y, min.z),
            Vector3::new(min.x, min.y, max.z),
            Vector3::new(min.x, max.y, min.z),
            Vector3::new(min.x, max.y, max.z),
            Vector3::new(max.x, min.y, min.z),
            Vector3::new(max.x, min.y, max.z),
            Vector3::new(max.x, max.y, min.z),
            Vector3::new(max.x, max.y, max.z),
        ]
    }

    fn collect_faces(&self) -> Vec<super::Face> {
        vec![
            // Bottom
            Face::new(vec![
                Vector3::new(self.min().x, self.min().y, self.min().z),
                Vector3::new(self.max().x, self.min().y, self.min().z),
                Vector3::new(self.max().x, self.min().y, self.max().z),
                Vector3::new(self.min().x, self.min().y, self.max().z),
            ]),
            // Top
            Face::new(vec![
                Vector3::new(self.min().x, self.max().y, self.min().z),
                Vector3::new(self.max().x, self.max().y, self.min().z),
                Vector3::new(self.max().x, self.max().y, self.max().z),
                Vector3::new(self.min().x, self.max().y, self.max().z),
            ]),
            // Front
            Face::new(vec![
                Vector3::new(self.min().x, self.min().y, self.min().z),
                Vector3::new(self.max().x, self.min().y, self.min().z),
                Vector3::new(self.max().x, self.max().y, self.min().z),
                Vector3::new(self.min().x, self.max().y, self.min().z),
            ]),
            // Back
            Face::new(vec![
                Vector3::new(self.min().x, self.min().y, self.max().z),
                Vector3::new(self.max().x, self.min().y, self.max().z),
                Vector3::new(self.max().x, self.max().y, self.max().z),
                Vector3::new(self.min().x, self.max().y, self.max().z),
            ]),
            // Left
            Face::new(vec![
                Vector3::new(self.min().x, self.min().y, self.min().z),
                Vector3::new(self.min().x, self.min().y, self.max().z),
                Vector3::new(self.min().x, self.max().y, self.max().z),
                Vector3::new(self.min().x, self.max().y, self.min().z),
            ]),
            // Right
            Face::new(vec![
                Vector3::new(self.max().x, self.min().y, self.min().z),
                Vector3::new(self.max().x, self.min().y, self.max().z),
                Vector3::new(self.max().x, self.max().y, self.max().z),
                Vector3::new(self.max().x, self.max().y, self.min().z),
            ]),
        ]
    }
}

impl std::fmt::Debug for AABB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AABB: min: {:?}, max: {:?}", self.min(), self.max())
    }
}
