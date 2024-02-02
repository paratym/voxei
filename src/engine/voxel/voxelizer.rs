use nalgebra::Vector3;

use crate::engine::{
    geometry::shapes::{aabb::AABB, Shape},
    model::mesh::Mesh,
};

use super::{
    morton,
    octree::{VoxelData, VoxelSVO, VoxelSVOBuilder},
};

pub fn voxelize(mesh: &Mesh, subdivisions: u32) -> VoxelSVO {
    let grid_length = (1 << subdivisions) as u32;
    let morton_count = grid_length.pow(3);

    let bbox = mesh.bbox();
    let max_length = bbox.half_extents().max() * 2.0;

    // The conversion from 1 voxel unit * unit_length = world space length
    let unit_length = (1.0 / grid_length as f32) * max_length;

    println!("max_length: {:?}", max_length);
    println!("unit_length: {:?}", unit_length);
    println!("bbox: {:?}", bbox);

    let mut voxel_data: Vec<VoxelData> = Vec::new();
    let mut voxel_marker = vec![0 as u8; morton_count as usize];

    for triangle in mesh.triangles() {
        let local_min = Vector3::new(
            triangle.v1.x.min(triangle.v2.x.min(triangle.v3.x)) - bbox.min().x,
            triangle.v1.y.min(triangle.v2.y.min(triangle.v3.y)) - bbox.min().y,
            triangle.v1.z.min(triangle.v2.z.min(triangle.v3.z)) - bbox.min().z,
        );
        let local_max = Vector3::new(
            triangle.v1.x.max(triangle.v2.x.max(triangle.v3.x)) - bbox.min().x,
            triangle.v1.y.max(triangle.v2.y.max(triangle.v3.y)) - bbox.min().y,
            triangle.v1.z.max(triangle.v2.z.max(triangle.v3.z)) - bbox.min().z,
        );

        let map_to_grid = |vec: Vector3<f32>| -> Vector3<u32> {
            let vec = vec * (grid_length as f32 / max_length) as f32;
            let vec = Vector3::new(
                vec.x.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
                vec.y.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
                vec.z.floor().clamp(0.0, grid_length as f32 - 1.0) as u32,
            );
            Vector3::new(vec.x, grid_length - 1 - vec.y, grid_length - 1 - vec.z)
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

        let normal = (triangle.v2 - triangle.v1)
            .cross(&(triangle.v3 - triangle.v1))
            .normalize();

        let square_bbox = {
            let half_max_length = max_length / 2.0;
            AABB::new(
                bbox.center(),
                Vector3::new(half_max_length, half_max_length, half_max_length),
            )
        };
        let world_grid_min = square_bbox.min();
        println!("world_grid_min: {:?}", world_grid_min);
        // Iterate through triangle grid voxels
        for x in min_grid.x..=max_grid.x {
            for y in min_grid.y..=max_grid.y {
                for z in min_grid.z..=max_grid.z {
                    let index = morton::util::morton_encode(x, y, z);

                    if voxel_marker[index as usize] == 1 {
                        continue;
                    }

                    let local_min: Vector3<f32> = world_grid_min
                        + Vector3::new(
                            x as f32 * unit_length,
                            y as f32 * unit_length,
                            z as f32 * unit_length,
                        );
                    let local_max = local_min.map(|x| x + unit_length);
                    let local_aabb = AABB::new(local_min, local_max);

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
    println!("voxel_data: {:?}", voxel_data.len());

    // Create SVO
    let mut builder = VoxelSVOBuilder::new(grid_length as usize);
    voxel_data.iter().for_each(|x| builder.add_voxel(x.clone()));
    builder.finalize_svo()
}
