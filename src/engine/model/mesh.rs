use nalgebra::Vector3;

use crate::engine::geometry::shapes::{aabb::AABB, triangle::Triangle};

pub struct Mesh {
    triangles: Vec<Triangle>,
    bbox: AABB,
}

impl Mesh {
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

        for triangle in &triangles {
            min.x = min
                .x
                .min(triangle.v1.x.min(triangle.v2.x.min(triangle.v3.x)));
            min.y = min
                .y
                .min(triangle.v1.y.min(triangle.v2.y.min(triangle.v3.y)));
            min.z = min
                .z
                .min(triangle.v1.z.min(triangle.v2.z.min(triangle.v3.z)));

            max.x = max
                .x
                .max(triangle.v1.x.max(triangle.v2.x.max(triangle.v3.x)));
            max.y = max
                .y
                .max(triangle.v1.y.max(triangle.v2.y.max(triangle.v3.y)));
            max.z = max
                .z
                .max(triangle.v1.z.max(triangle.v2.z.max(triangle.v3.z)));
        }

        Mesh {
            triangles,
            bbox: AABB::new_min_max(min, max),
        }
    }

    pub fn triangles(&self) -> &Vec<Triangle> {
        &self.triangles
    }

    pub fn bbox(&self) -> &AABB {
        &self.bbox
    }
}

impl From<&tobj::Model> for Mesh {
    fn from(model: &tobj::Model) -> Self {
        let mut triangles = Vec::new();
        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

        for i in (0..model.mesh.indices.len()).step_by(3) {
            let i0 = model.mesh.indices[i] as usize;
            let i1 = model.mesh.indices[i + 1] as usize;
            let i2 = model.mesh.indices[i + 2] as usize;

            let v0 = Vector3::new(
                model.mesh.positions[i0 * 3],
                model.mesh.positions[i0 * 3 + 1],
                model.mesh.positions[i0 * 3 + 2],
            );
            let v1 = Vector3::new(
                model.mesh.positions[i1 * 3],
                model.mesh.positions[i1 * 3 + 1],
                model.mesh.positions[i1 * 3 + 2],
            );
            let v2 = Vector3::new(
                model.mesh.positions[i2 * 3],
                model.mesh.positions[i2 * 3 + 1],
                model.mesh.positions[i2 * 3 + 2],
            );

            let triangle = Triangle::new(v0, v1, v2);
            triangles.push(triangle);

            min.x = min.x.min(v0.x.min(v1.x.min(v2.x)));
            min.y = min.y.min(v0.y.min(v1.y.min(v2.y)));
            min.z = min.z.min(v0.z.min(v1.z.min(v2.z)));

            max.x = max.x.max(v0.x.max(v1.x.max(v2.x)));
            max.y = max.y.max(v0.y.max(v1.y.max(v2.y)));
            max.z = max.z.max(v0.z.max(v1.z.max(v2.z)));
        }

        Mesh {
            triangles,
            bbox: AABB::new_min_max(min, max),
        }
    }
}

impl From<&Vec<tobj::Model>> for Mesh {
    fn from(models: &Vec<tobj::Model>) -> Self {
        let meshes = models
            .into_iter()
            .map(|model| Mesh::from(model))
            .collect::<Vec<Mesh>>();

        let mut triangles = Vec::new();
        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

        for mesh in meshes {
            let bbox = mesh.bbox;
            triangles.extend(mesh.triangles);

            min.x = min.x.min(bbox.min().x);
            min.y = min.y.min(bbox.min().y);
            min.z = min.z.min(bbox.min().z);

            max.x = max.x.max(bbox.max().x);
            max.y = max.y.max(bbox.max().y);
            max.z = max.z.max(bbox.max().z);
        }

        Mesh {
            triangles,
            bbox: AABB::new_min_max(min, max),
        }
    }
}
