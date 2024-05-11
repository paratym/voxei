use nalgebra::{Point3, Vector3};

use crate::engine::common::transform::Transform;

use super::shapes::aabb::AABB;

pub struct Ray {
    position: Point3<f32>,
    direction: Vector3<f32>,
    inv_direction: Vector3<f32>,
}

impl Ray {
    pub fn new(position: Point3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            position,
            direction,
            inv_direction: Vector3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z)
                .map(|x| {
                    if x.is_infinite() || x.is_nan() {
                        1.0e10
                    } else {
                        x
                    }
                }),
        }
    }

    pub fn position(&self) -> Point3<f32> {
        self.position
    }

    pub fn direction(&self) -> Vector3<f32> {
        self.direction
    }

    pub fn direction_sign(&self) -> Vector3<f32> {
        self.direction.map(|x| x.signum())
    }

    pub fn inv_direction(&self) -> Vector3<f32> {
        self.inv_direction
    }

    pub fn traverse(&self, t: f32) -> Point3<f32> {
        self.position + t * self.direction
    }

    // Returns the t value the ray intersects the aabb, None if there is no intersection.
    pub fn intersect_aabb(&self, aabb: &AABB) -> Option<f32> {
        let t0 = (aabb.min() - self.position).component_mul(&self.inv_direction);
        let t1 = (aabb.max() - self.position).component_mul(&self.inv_direction);
        let tmin = t0.zip_map(&t1, f32::min).max().max(0.0);
        let tmax = t0.zip_map(&t1, f32::max).min();

        return if tmax >= tmin { Some(tmin) } else { None };
    }
}

impl From<Transform> for Ray {
    fn from(transform: Transform) -> Self {
        Self::new(
            transform.isometry.translation.vector.into(),
            transform.isometry.rotation * Vector3::z(),
        )
    }
}

impl From<&Transform> for Ray {
    fn from(transform: &Transform) -> Self {
        Self::new(
            transform.isometry.translation.vector.into(),
            transform.isometry.rotation * Vector3::z(),
        )
    }
}
