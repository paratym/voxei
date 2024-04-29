use nalgebra::Vector3;

pub mod chunk_generator;
pub mod dynamic_world;
pub mod static_world;
pub mod vox_world;

pub mod vox_constants {
    pub const VOXEL_WORLD_LENGTH: f32 = 1.0;

    pub const BRICK_LENGTH: usize = 8;
    pub const BRICK_AREA: usize = BRICK_LENGTH * BRICK_LENGTH;
    pub const BRICK_VOLUME: usize = BRICK_AREA * BRICK_LENGTH;
    pub const BRICK_WORLD_LENGTH: f32 = BRICK_LENGTH as f32 * VOXEL_WORLD_LENGTH;
    pub const BRICK_MORTON_LENGTH: u64 = BRICK_LENGTH.trailing_zeros() as u64 * 3;

    pub const CHUNK_LENGTH: usize = 8;
    pub const CHUNK_AREA: usize = CHUNK_LENGTH * CHUNK_LENGTH;
    pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_LENGTH;

    pub const CHUNK_VOXEL_LENGTH: usize = CHUNK_LENGTH * BRICK_LENGTH;
    pub const CHUNK_WORLD_LENGTH: f32 = CHUNK_VOXEL_LENGTH as f32 * VOXEL_WORLD_LENGTH;

    pub const SUPER_CHUNK_LENGTH: usize = 4;
    pub const SUPER_CHUNK_AREA: usize = SUPER_CHUNK_LENGTH * SUPER_CHUNK_LENGTH;
    pub const SUPER_CHUNK_VOLUME: usize = SUPER_CHUNK_AREA * SUPER_CHUNK_LENGTH;
}

pub mod util {
    use nalgebra::Vector3;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Morton(u64);

    impl Morton {
        pub fn new(value: u64) -> Self {
            Morton(value)
        }

        /// Splits the first 21 bits by inserting two 0s to the left of each bit.
        fn split(x: u32) -> u64 {
            let mut x = x as u64 & 0x1f_ffff; //            0000000000000000000000000000000000000000000111111111111111111111
            x = (x | (x << 32)) & 0x001f_0000_0000_ffff; // 0000000000011111000000000000000000000000000000001111111111111111
            x = (x | (x << 16)) & 0x001f_0000_ff00_00ff; // 0000000000011111000000000000000011111111000000000000000011111111
            x = (x | (x << 8)) & 0x100f_00f0_0f00_f00f; //  0000000100001111000000001111000000001111000000001111000000001111
            x = (x | (x << 4)) & 0x10c3_0c30_c30c_30c3; //  0001000011000011000011000011000011000011000011000011000011000011
            x = (x | (x << 2)) & 0x1249_2492_4924_9249; //  0001001001001001001001001001001001001001001001001001001001001001
            x
        }

        /// Compacts the starting with bit 0 skipping every two bits resulting in a 21 bit result.
        fn compact(x: u64) -> u32 {
            let mut x = x & 0x1249_2492_4924_9249; //       0001001001001001001001001001001001001001001001001001001001001001
            x = (x | (x >> 2)) & 0x10c3_0c30_c30c_30c3; //  0001000011000011000011000011000011000011000011000011000011000011
            x = (x | (x >> 4)) & 0x100f_00f0_0f00_f00f; //  0000000100001111000000001111000000001111000000001111000000001111
            x = (x | (x >> 8)) & 0x001f_0000_ff00_00ff; //  0000000000011111000000000000000011111111000000000000000011111111
            x = (x | (x >> 16)) & 0x001f_0000_0000_ffff; // 0000000000011111000000000000000000000000000000001111111111111111
            x = (x | (x >> 32)) & 0x1f_ffff; //             0000000000000000000000000000000000000000000111111111111111111111
            x as u32
        }

        pub fn encode(position: Vector3<u32>) -> Self {
            Morton(
                Self::split(position.x)
                    | (Self::split(position.y) << 1)
                    | (Self::split(position.z) << 2),
            )
        }

        pub fn decode(&self) -> Vector3<u64> {
            Vector3::new(
                Self::compact(self.0) as u64,
                Self::compact(self.0 >> 1) as u64,
                Self::compact(self.0 >> 2) as u64,
            )
        }
    }

    impl std::ops::Deref for Morton {
        type Target = u64;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub fn next_pow2(mut value: u32) -> u32 {
        value -= 1;
        value |= value >> 1;
        value |= value >> 2;
        value |= value >> 4;
        value |= value >> 8;
        value |= value >> 16;
        value + 1
    }
}
