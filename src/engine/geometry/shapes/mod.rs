use nalgebra::Vector3;

pub mod aabb;
pub mod triangle;

pub type Vertex = Vector3<f32>;
pub struct Face {
    vertices: Vec<Vertex>,
    normal: Vector3<f32>,
}

impl Face {
    pub fn new(vertices: Vec<Vertex>) -> Self {
        // TODO: Ensure vertices coplanar, remove degenrate vertices and sort for a winding order
        let normal = (vertices[1] - vertices[0])
            .cross(&(vertices[2] - vertices[0]))
            .normalize();

        Self { vertices, normal }
    }

    /// Calculates the separating axis theorem axes to test for each face
    pub fn calculate_sat_axes(&self) -> Vec<Vector3<f32>> {
        let mut axes = vec![self.normal];
        for i in 0..self.vertices.len() {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % self.vertices.len()];
            axes.push((v2 - v1).cross(&self.normal).normalize());
        }
        axes
    }
}

pub struct Projection {
    min: f32,
    max: f32,
}

impl Projection {
    pub fn overlap(&self, other: &Projection) -> bool {
        self.min <= other.max && self.max >= other.min
    }
}

pub trait Shape {
    fn collect_vertices(&self) -> Vec<Vertex>;
    fn collect_faces(&self) -> Vec<Face>;

    fn project(&self, axis: &Vector3<f32>) -> Projection {
        let mut min = std::f32::MAX;
        let mut max = std::f32::MIN;

        for vertex in self.collect_vertices() {
            let projection = vertex.dot(&axis);
            min = min.min(projection);
            max = max.max(projection);
        }

        Projection { min, max }
    }
    fn test_intersection(&self, other: &dyn Shape) -> bool {
        let axes = self
            .collect_faces()
            .into_iter()
            .chain(other.collect_faces().into_iter())
            .map(|face| face.calculate_sat_axes())
            .flatten()
            .collect::<Vec<_>>();

        for axis in axes {
            let p1 = self.project(&axis);
            let p2 = other.project(&axis);

            if !p1.overlap(&p2) {
                return false;
            }
        }

        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_intersection() {
        let t1 = triangle::Triangle::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let t2 = triangle::Triangle::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );

        assert!(t1.test_intersection(&t2));
    }

    #[test]
    fn test_aabb_intersection() {
        let aabb1 = aabb::AABB::new_center_half_extent(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 1.0),
        );
        let aabb2 = aabb::AABB::new_center_half_extent(
            Vector3::new(0.5, 0.5, 0.5),
            Vector3::new(1.0, 1.0, 1.0),
        );

        assert!(aabb1.test_intersection(&aabb2));
    }

    #[test]
    fn test_triangle_aabb_intersection() {
        let aabb = aabb::AABB::new_center_half_extent(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 1.0),
        );
        let t = triangle::Triangle::new(
            Vector3::new(0.5, 0.5, 0.5),
            Vector3::new(2.0, 0.5, 0.5),
            Vector3::new(0.5, 2.0, 0.5),
        );

        assert!(aabb.test_intersection(&t));
    }
}
