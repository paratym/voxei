use nalgebra::Vector3;

use super::{
    morton,
    octree::{SVONode, VoxelData, VoxelSVO, VoxelSVOBuilder},
};

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

pub struct TriReader {
    pub triangles: Vec<Triangle>,
    pub bbox: (Vector3<f32>, Vector3<f32>),
}

impl TriReader {
    pub fn new(models: &Vec<tobj::Model>) -> Self {
        let mut min = Vector3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX);
        let mut max = Vector3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN);

        let mut triangles = Vec::new();
        for model in models {
            let mesh = &model.mesh;
            for i in (0..mesh.indices.len()).step_by(3) {
                let v1 = Vector3::new(
                    mesh.positions[mesh.indices[i] as usize * 3],
                    mesh.positions[mesh.indices[i] as usize * 3 + 1],
                    mesh.positions[mesh.indices[i] as usize * 3 + 2],
                );
                let v2 = Vector3::new(
                    mesh.positions[mesh.indices[i + 1] as usize * 3],
                    mesh.positions[mesh.indices[i + 1] as usize * 3 + 1],
                    mesh.positions[mesh.indices[i + 1] as usize * 3 + 2],
                );
                let v3 = Vector3::new(
                    mesh.positions[mesh.indices[i + 2] as usize * 3],
                    mesh.positions[mesh.indices[i + 2] as usize * 3 + 1],
                    mesh.positions[mesh.indices[i + 2] as usize * 3 + 2],
                );

                min = min.map(|x| x.min(v1.x.min(v2.x.min(v3.x))));
                min = min.map(|x| x.min(v1.y.min(v2.y.min(v3.y))));
                min = min.map(|x| x.min(v1.z.min(v2.z.min(v3.z))));
                max = max.map(|x| x.max(v1.x.max(v2.x.max(v3.x))));
                max = max.map(|x| x.max(v1.y.max(v2.y.max(v3.y))));
                max = max.map(|x| x.max(v1.z.max(v2.z.max(v3.z))));

                triangles.push(Triangle { v1, v2, v3 });
            }
        }
        Self {
            triangles,
            bbox: (min, max),
        }
    }

    pub fn next(&mut self) -> Option<Triangle> {
        self.triangles.pop()
    }

    pub fn has_next(&self) -> bool {
        !self.triangles.is_empty()
    }
}

impl Iterator for TriReader {
    type Item = Triangle;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

pub struct Voxelizer {
    pub reader: TriReader,
    pub grid_length: u32,
}

impl Voxelizer {
    pub fn new(reader: TriReader, grid_length: u32) -> Self {
        if grid_length == 0 {
            panic!("Grid size must be greater than 0");
        }
        if grid_length & (grid_length - 1) != 0 {
            panic!("Grid size must be a power of 2");
        }

        Self {
            reader,
            grid_length,
        }
    }

    pub fn voxelize(mut self) -> VoxelSVO {
        let morton_count = self.grid_length.pow(3);

        let mut voxel_data: Vec<VoxelData> = Vec::new();
        let mut voxel_marker = vec![0 as u8; morton_count as usize];

        let bbox = self.reader.bbox;
        let max_length = Vector3::new(
            bbox.1.x - bbox.0.x,
            bbox.1.y - bbox.0.y,
            bbox.1.z - bbox.0.z,
        )
        .abs()
        .max();
        // triangle local coordinate * unit_length = clamp(grid coordinate, 0, grid_length - 1)
        let unit_length = self.grid_length as f32 / max_length;
        let inv_unit_length = 1.0 / unit_length;
        println!("grid_length: {:?}", self.grid_length);

        while self.reader.has_next() {
            let triangle = self.reader.next().unwrap();

            let local_min = Vector3::new(
                triangle.v1.x.min(triangle.v2.x.min(triangle.v3.x)) - bbox.0.x,
                triangle.v1.y.min(triangle.v2.y.min(triangle.v3.y)) - bbox.0.y,
                triangle.v1.z.min(triangle.v2.z.min(triangle.v3.z)) - bbox.0.z,
            ) / max_length;
            let local_max = Vector3::new(
                triangle.v1.x.max(triangle.v2.x.max(triangle.v3.x)) - bbox.0.x,
                triangle.v1.y.max(triangle.v2.y.max(triangle.v3.y)) - bbox.0.y,
                triangle.v1.z.max(triangle.v2.z.max(triangle.v3.z)) - bbox.0.z,
            ) / max_length;

            let map_to_grid = |vec: Vector3<f32>| -> Vector3<u32> {
                let vec = vec * unit_length as f32;
                println!("vec: {:?}", vec);
                let vec = Vector3::new(
                    vec.x.floor().clamp(0.0, self.grid_length as f32 - 1.0) as u32,
                    vec.y.floor().clamp(0.0, self.grid_length as f32 - 1.0) as u32,
                    vec.z.floor().clamp(0.0, self.grid_length as f32 - 1.0) as u32,
                );
                Vector3::new(
                    vec.x,
                    self.grid_length - 1 - vec.y,
                    self.grid_length - 1 - vec.z,
                )
            };

            let mut min_grid = map_to_grid(local_min);
            let mut max_grid = map_to_grid(local_max);

            // Since we flipped the y and z axis, we need to swap the min and max if there are now in the wrong order for the loops
            if min_grid.y > max_grid.y {
                std::mem::swap(&mut min_grid.y, &mut max_grid.y);
            }
            if min_grid.z > max_grid.z {
                std::mem::swap(&mut min_grid.z, &mut max_grid.z);
            }

            println!("local_min: {:?}", local_min);
            println!("local_max: {:?}", local_max);

            println!("min_grid: {:?}", min_grid);
            println!("max_grid: {:?}", max_grid);

            // Iterate through triangle grid voxels
            for x in min_grid.x..=max_grid.x {
                println!("x: {}", x);
                for y in min_grid.y..=max_grid.y {
                    println!("y: {}", y);
                    for z in min_grid.z..=max_grid.z {
                        println!("z: {}", z);

                        let index = morton::util::morton_encode(x, y, z);

                        // Check if voxel is already marked
                        if voxel_marker[index as usize] == 1 {
                            println!("Voxel already marked");
                            continue;
                        }

                        let point = Vector3::new(x as f32, y as f32, z as f32) * unit_length;

                        let is_intersecting = true;
                        // TODO - clacualte the triangle aabb intersection

                        if is_intersecting {
                            let normal = triangle.v1.cross(&(triangle.v2 - triangle.v1));

                            voxel_marker[index as usize] = 1;
                            voxel_data.push(VoxelData {
                                morton_code: index,
                                normal: normal.normalize().into(),
                            });
                            println!("Voxel: {:?}", point);
                        }
                    }
                }
            }
        }

        // Sort voxel data by morton code and build svo
        voxel_data.sort_by(|a, b| a.morton_code.cmp(&b.morton_code));

        let mut builder = VoxelSVOBuilder::new(self.grid_length as usize);
        voxel_data.iter().for_each(|x| builder.add_voxel(x.clone()));

        builder.finalize_svo(unit_length)
    }
}
