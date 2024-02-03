use nalgebra::Vector3;

use crate::engine::{
    geometry::shapes::{aabb::AABB, Shape},
    model::mesh::Mesh,
};

use super::{
    morton,
    octree::{VoxelData, VoxelSVO, VoxelSVOBuilder},
};

#[derive(Debug)]
pub struct VoxelizeResult {
    pub voxel_svo: VoxelSVO,

    /// The offset of the mesh's min point to the min point of the voxel grid
    /// This is due to the fact that voxel grids must be cubes so some axis may have grown.
    pub root_min_offset: Vector3<f32>,
}

// Margins in the percentage of the voxel size that will be used
// to pad the voxel for intersection tests
pub fn voxelize(mesh: &Mesh, subdivisions: u32, margin: f32) -> VoxelizeResult {
    let grid_length = (1 << subdivisions) as u32;
    let morton_count = grid_length.pow(3);

    let bbox = mesh.bbox();
    let max_length = bbox.half_extents().max() * 2.0;
    let bbox_scaling = 1.0 / max_length;
    let square_bbox = {
        let half_max_length = max_length / 2.0;
        println!("half_max_length: {:?}", half_max_length);
        AABB::new_center_half_extent(
            bbox.center(),
            Vector3::new(half_max_length, half_max_length, half_max_length),
        )
    };
    // How much each axis grew to result in a square bbox
    let bbox_offset = (square_bbox.half_extents() - bbox.half_extents()) * 2.0;
    let bbox_grid_offset = bbox_offset * (grid_length as f32 / max_length);
    println!("bbox_grid_offset: {:?}", bbox_grid_offset);
    let bbox_grid_offset = Vector3::new(
        bbox_grid_offset.x.floor() as u32,
        bbox_grid_offset.y.floor() as u32,
        bbox_grid_offset.z.floor() as u32,
    );

    // The conversion from 1 voxel unit * unit_length = world space length
    let unit_length = (1.0 / grid_length as f32) * max_length;
    let scaled_margin = margin * unit_length;

    println!("max_length: {:?}", max_length);
    println!("unit_length: {:?}", unit_length);
    println!("bbox: {:?}", bbox);
    println!("square_bbox: {:?}", square_bbox);
    println!("bbox_offset: {:?}", bbox_offset);
    println!("bbox_grid_offset: {:?}", bbox_grid_offset);

    let mut voxel_data: Vec<VoxelData> = Vec::new();
    let mut voxel_marker = vec![0 as u8; morton_count as usize];

    for triangle in mesh.triangles() {
        let tri_min = Vector3::new(
            triangle.v1.x.min(triangle.v2.x.min(triangle.v3.x)),
            triangle.v1.y.min(triangle.v2.y.min(triangle.v3.y)),
            triangle.v1.z.min(triangle.v2.z.min(triangle.v3.z)),
        );
        let tri_max = Vector3::new(
            triangle.v1.x.max(triangle.v2.x.max(triangle.v3.x)),
            triangle.v1.y.max(triangle.v2.y.max(triangle.v3.y)),
            triangle.v1.z.max(triangle.v2.z.max(triangle.v3.z)),
        );

        let world_min_anchor = square_bbox.min();

        let world_to_grid = |mut vec: Vector3<f32>| -> Vector3<u32> {
            // Convert vector from world space to model space, coordinate should now range from 0 to max_length
            vec -= world_min_anchor;
            println!("vec: {:?}", vec);
            // Scale the vector to the grid length
            vec /= unit_length;
            println!("vec: {:?}", vec);
            let vec = Vector3::new(
                vec.x.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
                vec.y.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
                vec.z.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
            );
            Vector3::new(vec.x, grid_length - 1 - vec.y, grid_length - 1 - vec.z)
        };

        let world_min_max_to_grid = |min: Vector3<f32>, max: Vector3<f32>| {
            let mut min = world_to_grid(min);
            let mut max = world_to_grid(max);

            // Since we flip for the grid.
            if min.y > max.y {
                std::mem::swap(&mut min.y, &mut max.y);
            }
            if min.z > max.z {
                std::mem::swap(&mut min.z, &mut max.z);
            }

            (min, max)
        };

        let (min_grid, max_grid) = world_min_max_to_grid(tri_min, tri_max);

        println!("tri_min: {:?}", tri_min);
        println!("tri_max: {:?}", tri_max);
        println!("min_grid: {:?}", min_grid);
        println!("max_grid: {:?}", max_grid);

        let normal = (triangle.v2 - triangle.v1)
            .cross(&(triangle.v3 - triangle.v1))
            .normalize();

        // Iterate through triangle grid voxels
        for x in min_grid.x..=max_grid.x {
            for y in min_grid.y..=max_grid.y {
                for z in min_grid.z..=max_grid.z {
                    println!("x: {}, y: {}, z: {}", x, y, z);
                    let index = morton::util::morton_encode(x, y, z);

                    if voxel_marker[index as usize] == 1 {
                        continue;
                    }

                    // // Define intersection padding for the voxel
                    let mbottom = y > min_grid.y;
                    let mtop = y < max_grid.y;
                    let mleft = x > min_grid.x;
                    let mright = x < max_grid.x;
                    let mback = z > min_grid.z;
                    let mfront = z < max_grid.z;

                    // if mtop {
                    //     voxel_world_max.y -= scaled_margin;
                    // }
                    // if mbottom {
                    //     voxel_world_min.y += scaled_margin;
                    // }
                    // if mleft {
                    //     voxel_world_min.x += scaled_margin;
                    // }
                    // if mright {
                    //     voxel_world_max.x -= scaled_margin;
                    // }
                    // if mfront {
                    //     voxel_world_max.z -= scaled_margin;
                    // }
                    // if mback {
                    //     voxel_world_min.z += scaled_margin;
                    // }

                    // let voxel_world_aabb = AABB::new_min_max(voxel_world_min, voxel_world_max);

                    if true {
                        voxel_marker[index as usize] = 1;
                        voxel_data.push(VoxelData {
                            morton_code: index,
                            normal: normal.into(),
                        });
                    }
                }
            }
        }
    }

    // Sort voxel data by morton code and build svo
    voxel_data.sort_by(|a, b| a.morton_code.cmp(&b.morton_code));

    // Create SVO
    let mut builder = VoxelSVOBuilder::new(grid_length as usize);
    voxel_data.iter().for_each(|x| builder.add_voxel(x.clone()));

    let root_unit_offset = Vector3::new(
        bbox_offset.x * bbox_scaling,
        bbox_offset.y * bbox_scaling,
        bbox_offset.z * bbox_scaling,
    );

    VoxelizeResult {
        voxel_svo: builder.finalize_svo(),
        root_min_offset: root_unit_offset,
    }
}
