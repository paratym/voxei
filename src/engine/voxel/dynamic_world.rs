use nalgebra::{SimdPartialOrd, Vector3};

use crate::{engine::voxel::vox_constants::CHUNK_VOLUME, settings::Settings};

use super::{
    chunk_generator::GeneratedChunk,
    util::{next_pow2, Morton},
    vox_constants::{BRICK_AREA, BRICK_VOLUME, SUPER_CHUNK_VOLUME},
    vox_world::{ChunkRadius, DynChunkPos, WorldChunkPos},
};

/// Our voxel world representation for rendering and is more easily editable due to the flat array
/// structure for each brick and masks for each hierarchy level.
pub struct DynVoxelWorld {
    super_chunk_grid_mask: BitGridMask,
    chunk_occupancy_mask: GridMask,
    chunk_bit_mask: BitGridMask,
    chunk_normal_grid: BitGridMask,
    brick_indices_grid: BrickIndexGrid,
    brick_data: BrickDataList,
    brick_palette_data: BrickPaletteList,

    brick_changes: Vec<BrickChange>,
    brick_normal_updates: Vec<BrickChange>,

    chunk_render_distance: ChunkRadius,

    /// The logical local translation we perform so memory can stay in place as we change origins.
    chunk_translation: Vector3<i32>,
}

impl DynVoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        let chunk_render_volume = settings.chunk_render_distance.pow2_volume();
        println!("Chunk render volume: {}", chunk_render_volume);
        let super_chunk_render_volume =
            (chunk_render_volume as f32 / SUPER_CHUNK_VOLUME as f32).ceil() as u64;
        println!("Super chunk render volume: {}", super_chunk_render_volume);
        let brick_render_volume = chunk_render_volume * BRICK_VOLUME as u64;
        println!("Brick render volume: {}", brick_render_volume);

        Self {
            super_chunk_grid_mask: BitGridMask::new(super_chunk_render_volume as usize),
            chunk_occupancy_mask: GridMask::new(chunk_render_volume as usize),
            chunk_bit_mask: BitGridMask::new(chunk_render_volume as usize),
            chunk_normal_grid: BitGridMask::new(chunk_render_volume as usize),
            brick_indices_grid: BrickIndexGrid::new(brick_render_volume as usize),
            brick_data: BrickDataList::new(),
            brick_palette_data: BrickPaletteList::new(),

            brick_changes: Vec::new(),
            brick_normal_updates: Vec::new(),

            chunk_render_distance: settings.chunk_render_distance,
            chunk_translation: Vector3::zeros(),
        }
    }

    pub fn update_translation(
        &mut self,
        chunk_translation: Vector3<i32>,
        old_chunk_center: WorldChunkPos,
    ) {
        let slm = self.chunk_render_distance.pow2_side_length() as i32;
        let new_chunk_center = old_chunk_center.vector + chunk_translation;
        let dyn_translation = Vector3::new(
            new_chunk_center.x.rem_euclid(slm),
            new_chunk_center.y.rem_euclid(slm),
            new_chunk_center.z.rem_euclid(slm),
        );
        let old_dyn_translation = self.chunk_translation;
        self.chunk_translation = dyn_translation;

        // Calculates on each axis the range of values on that axis need to be unloaded, the reference, my whiteboard
        let unloaded =
            chunk_translation.zip_zip_map(&dyn_translation, &old_dyn_translation, |t, new, old| {
                if t.is_positive() {
                    (new - t)..new
                } else {
                    (old + t)..old
                }
            });

        for x in unloaded.x.to_owned() {
            let x = x.rem_euclid(slm) as u32;
            for y in 0..(slm as u32) {
                for z in 0..(slm as u32) {
                    self.unload_chunk(DynChunkPos::new(x, y, z));
                }
            }
        }

        for y in unloaded.y.to_owned() {
            let y = y.rem_euclid(slm) as u32;
            for x in 0..(slm as u32) {
                for z in 0..(slm as u32) {
                    self.unload_chunk(DynChunkPos::new(x, y, z));
                }
            }
        }

        for z in unloaded.z.to_owned() {
            let z = z.rem_euclid(slm) as u32;
            for x in 0..(slm as u32) {
                for y in 0..(slm as u32) {
                    self.unload_chunk(DynChunkPos::new(x, y, z));
                }
            }
        }
    }

    fn unload_chunk(&mut self, local_chunk_pos: DynChunkPos) {
        let morton = local_chunk_pos.morton();
        self.chunk_occupancy_mask
            .set_status(morton, SpatialStatus::Unloaded);
        self.chunk_bit_mask.set_status(morton, false);
        self.chunk_normal_grid.set_status(morton, false);
    }

    pub fn chunk_status(&self, local_chunk_pos: DynChunkPos) -> SpatialStatus {
        let morton = Morton::encode(local_chunk_pos.vector);
        self.chunk_occupancy_mask.status(morton)
    }

    pub fn is_brick_loaded(&self, morton: u64) -> bool {
        self.brick_indices_grid.0[morton as usize]
            .status()
            .is_loaded()
    }

    pub fn set_generated_chunk(&mut self, local_chunk_pos: DynChunkPos, chunk: GeneratedChunk) {
        let morton = local_chunk_pos.morton();
        if chunk.is_empty {
            self.chunk_occupancy_mask
                .set_status(morton, SpatialStatus::LoadedEmpty);
        } else {
            self.chunk_occupancy_mask
                .set_status(morton, SpatialStatus::Loaded);
            self.chunk_bit_mask.set_status(morton, true);
            let local_brick_min = local_chunk_pos.to_dyn_brick_pos();
            let local_brick_min_morton = *local_brick_min.morton();
            for brick_morton in 0..CHUNK_VOLUME {
                let generated_voxels = chunk.brick_data(Morton::new(brick_morton as u64));
                let is_empty = generated_voxels.iter().all(|voxel| voxel.is_none());

                let dyn_brick_morton = local_brick_min_morton + brick_morton as u64;
                if is_empty {
                    self.set_brick(dyn_brick_morton, None);
                } else {
                    let brick_data = BrickData::from_voxel_array(&generated_voxels);
                    let brick_palette = BrickPalette::from_voxel_array(&generated_voxels);
                    self.set_brick(dyn_brick_morton, Some((brick_data, brick_palette)));
                }
            }
        }
    }

    pub fn set_chunk_loading(&mut self, local_chunk_pos: DynChunkPos) {
        let morton = local_chunk_pos.morton();
        self.chunk_occupancy_mask
            .set_status(morton, SpatialStatus::Loading);
    }

    pub fn set_brick(&mut self, morton: u64, brick: Option<(BrickData, BrickPalette)>) {
        let brick_index = if let Some((mut brick_data, mut brick_material_data)) = brick {
            let size_i = match brick_material_data.next_pow_2_size() {
                64 => 0,
                128 => 1,
                256 => 2,
                512 => 3,
                _ => unreachable!(),
            };
            let indices = brick_material_data.indices.take();
            let material_index = self.brick_palette_data.insert(brick_material_data);
            brick_data.palette_index = material_index | (size_i << 30);
            let index = self.brick_data.insert(brick_data, *indices.unwrap());
            if index == 1347 {
                println!("palette index: {}", material_index);
            }
            BrickIndex::new_loaded(index)
        } else {
            BrickIndex::new_loaded_empty()
        };
        self.brick_changes.push(BrickChange {
            brick_morton: Morton::new(morton),
        });
        self.brick_indices_grid.0[morton as usize] = brick_index;
    }

    pub fn update_chunk_normals(&mut self, local_chunk_pos: DynChunkPos) {
        // self.brick_normal_updates.push(BrickChange {
        //     brick_morton: Morton::new(14682733),
        // });
        let morton = local_chunk_pos.morton();
        if self.chunk_occupancy_mask.status(morton) != SpatialStatus::Loaded {
            return;
        }

        self.chunk_normal_grid.set_status(morton, true);
        let local_brick_min = local_chunk_pos.to_dyn_brick_pos();
        let local_brick_min_morton = *local_brick_min.morton();
        for brick_morton in 0..CHUNK_VOLUME {
            let dyn_brick_morton = local_brick_min_morton + brick_morton as u64;
            if self.brick_indices_grid.0[dyn_brick_morton as usize].status()
                == SpatialStatus::Loaded
            {
                self.brick_normal_updates.push(BrickChange {
                    brick_morton: Morton::new(dyn_brick_morton),
                });
            }
        }
    }

    pub fn collect_brick_changes(&mut self) -> Vec<BrickChange> {
        std::mem::replace(&mut self.brick_changes, Vec::new())
    }

    pub fn collect_brick_normal_updates(&mut self) -> Vec<BrickChange> {
        std::mem::replace(&mut self.brick_normal_updates, Vec::new())
    }

    pub fn super_chunk_bit_grid(&self) -> &BitGridMask {
        &self.super_chunk_grid_mask
    }

    pub fn chunk_occupancy_grid(&self) -> &GridMask {
        &self.chunk_occupancy_mask
    }

    pub fn chunk_bit_grid(&self) -> &BitGridMask {
        &self.chunk_bit_mask
    }

    pub fn brick_indices_grid(&self) -> &BrickIndexGrid {
        &self.brick_indices_grid
    }

    pub fn brick_palette_list(&self) -> &BrickPaletteList {
        &self.brick_palette_data
    }

    pub fn chunk_render_distance(&self) -> ChunkRadius {
        self.chunk_render_distance
    }

    pub fn chunk_translation(&self) -> Vector3<u32> {
        self.chunk_translation.map(|x| x as u32)
    }

    pub fn brick_data(&self) -> &BrickDataList {
        &self.brick_data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialStatus {
    Unloaded = 0b00,
    Loading = 0b01,
    Loaded = 0b10,
    LoadedEmpty = 0b11,
}

impl SpatialStatus {
    pub fn is_loaded(&self) -> bool {
        match self {
            SpatialStatus::Loaded | SpatialStatus::LoadedEmpty => true,
            _ => false,
        }
    }
}

impl From<u16> for SpatialStatus {
    fn from(value: u16) -> Self {
        match value {
            0b00 => SpatialStatus::Unloaded,
            0b01 => SpatialStatus::Loading,
            0b10 => SpatialStatus::Loaded,
            0b11 => SpatialStatus::LoadedEmpty,
            _ => unreachable!(),
        }
    }
}

impl From<u32> for SpatialStatus {
    fn from(value: u32) -> Self {
        let value = value as u16;
        value.into()
    }
}

pub struct BrickIndexGrid(Vec<BrickIndex>);

impl BrickIndexGrid {
    pub fn new(volume: usize) -> Self {
        Self(vec![BrickIndex::new_unloaded(); volume])
    }

    pub fn as_slice(&self) -> &[BrickIndex] {
        &self.0
    }

    pub fn buffer_size(&self) -> usize {
        self.0.len() * std::mem::size_of::<BrickIndex>()
    }
}

// Every 2 bits is a status with the index being morton encoded.
pub struct GridMask(Vec<u16>);

impl GridMask {
    pub fn new(volume: usize) -> Self {
        Self(vec![0; volume / 8])
    }

    pub fn set_status(&mut self, morton: Morton, status: SpatialStatus) {
        let bit_index = (*morton & 0b111) * 2;
        // Clear the status bits
        self.0[(*morton >> 3) as usize] &= !(0b11 << bit_index);
        // Set the status bits
        self.0[(*morton >> 3) as usize] |= (status as u16) << bit_index;
    }

    pub fn status(&self, morton: Morton) -> SpatialStatus {
        let bit_index = (*morton & 0b111) * 2;
        let status = (self.0[(*morton >> 3) as usize] >> bit_index) & 0b11;

        status.into()
    }

    pub fn as_slice(&self) -> &[u16] {
        &self.0
    }

    pub fn buffer_size(&self) -> usize {
        self.0.len() * std::mem::size_of::<u16>()
    }
}

pub struct BitGridMask(Vec<u8>);

impl BitGridMask {
    pub fn new(volume: usize) -> Self {
        Self(vec![0; (volume as f32 / 8.0).ceil() as usize])
    }

    pub fn set_status(&mut self, morton: Morton, status: bool) {
        let bit_index = *morton & 0b111;
        // Clear the status bits
        self.0[(*morton >> 3) as usize] &= !(1 << bit_index);
        // Set the status bits
        self.0[(*morton >> 3) as usize] |= (status as u8) << bit_index;
    }

    pub fn status(&self, morton: Morton) -> bool {
        let bit_index = *morton & 0b111;
        let status = self.0[(*morton >> 3) as usize] >> bit_index;

        status == 1
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn buffer_size(&self) -> usize {
        self.0.len() * std::mem::size_of::<u8>()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BrickIndex(u32);

impl BrickIndex {
    pub fn new_unloaded() -> Self {
        Self(0)
    }

    pub fn new_loaded(index: u32) -> Self {
        Self((SpatialStatus::Loaded as u32) << 30 | (index & 0x3FFF_FFFF))
    }

    pub fn new_loaded_empty() -> Self {
        Self((SpatialStatus::LoadedEmpty as u32) << 30)
    }

    pub fn status(&self) -> SpatialStatus {
        (self.0 >> 30).into()
    }

    pub fn index(&self) -> u32 {
        self.0 & 0x3FFF_FFFF
    }
}

pub struct BrickDataList {
    free_head: u32,
    data: Vec<BrickData>,

    // Each index is u9
    palette_indices: Vec<u16>,
}

const NULL_FREE_INDEX: u32 = 0x7FFFFFFF;

impl BrickDataList {
    pub fn new() -> Self {
        Self {
            free_head: NULL_FREE_INDEX,
            data: Vec::new(),
            palette_indices: Vec::new(),
        }
    }

    /// Returns the brick list index for the inserted brick data.
    pub fn insert(
        &mut self,
        brick_data: BrickData,
        brick_palette_indices: [u16; BRICK_VOLUME],
    ) -> u32 {
        if self.free_head != NULL_FREE_INDEX {
            let new_index = self.free_head;
            self.free_head = self.data[self.free_head as usize].next_free();
            self.data[new_index as usize] = brick_data;
            let indices_index = new_index as usize * BRICK_VOLUME;
            self.palette_indices[indices_index..(indices_index + BRICK_VOLUME)]
                .copy_from_slice(&brick_palette_indices);

            return new_index;
        } else {
            self.data.push(brick_data);
            self.palette_indices
                .extend_from_slice(&brick_palette_indices);
            return self.data.len() as u32 - 1;
        }
    }

    pub fn get(&self, index: u32) -> &BrickData {
        &self.data[index as usize]
    }

    pub fn get_indices(&self, index: u32) -> &[u16] {
        let index = index as usize * BRICK_VOLUME;
        &self.palette_indices[index..(index + BRICK_VOLUME)]
    }
}

#[repr(C)]
union VoxelMask {
    voxel_mask: [u8; BRICK_AREA],
    next_free: u32,
}

#[repr(C)]
pub struct BrickData {
    voxel_mask: VoxelMask,
    palette_index: u32,
}

impl BrickData {
    pub fn new_free(next_free: u32) -> Self {
        Self {
            voxel_mask: VoxelMask {
                voxel_mask: [0; BRICK_AREA],
            },
            palette_index: 0,
        }
    }

    pub fn set_free(&mut self, next_free: u32) {
        self.voxel_mask = VoxelMask { next_free };
    }

    pub fn next_free(&self) -> u32 {
        unsafe { self.voxel_mask.next_free }
    }

    pub fn from_voxel_array(voxel_data: &Vec<Option<Vector3<f32>>>) -> Self {
        let mut voxel_mask = [0; BRICK_AREA];
        for i in 0..BRICK_VOLUME {
            let voxel = &voxel_data[i];
            if voxel.is_some() {
                voxel_mask[i >> 3] |= 1 << (i & 0b111);
            }
        }

        Self {
            voxel_mask: VoxelMask { voxel_mask },
            palette_index: 0,
        }
    }

    pub fn palette_index(&self) -> u32 {
        self.palette_index & 0x3FFF_FFFF
    }

    pub fn palette_size(&self) -> u32 {
        let size_bits = self.palette_index >> 30;
        match size_bits {
            0 => 64,
            1 => 128,
            2 => 256,
            3 => 512,
            _ => unreachable!(),
        }
    }
}

pub struct BrickChange {
    pub brick_morton: Morton,
}

pub struct BrickPalette {
    data: Vec<PackedVoxelMaterial>,
    indices: Option<Box<[u16; BRICK_VOLUME]>>,
}

impl BrickPalette {
    pub fn new(data: Vec<PackedVoxelMaterial>, indices: [u16; BRICK_VOLUME]) -> Self {
        if data.len() > 512 {
            panic!("Brick palette can only have a maximum of 512 entries");
        }
        Self {
            data,
            indices: Some(Box::new(indices)),
        }
    }

    pub fn from_voxel_array(voxel_data: &Vec<Option<Vector3<f32>>>) -> Self {
        let mut data = Vec::new();
        let mut indices = [0; BRICK_VOLUME];
        for i in 0..BRICK_VOLUME {
            let voxel = &voxel_data[i];
            if let Some(voxel) = voxel {
                data.push(PackedVoxelMaterial::new(
                    [voxel.x, voxel.y, voxel.z],
                    [0.0; 3],
                ));
                indices[i] = (data.len() - 1) as u16;
            }
        }

        Self::new(data, indices)
    }

    pub fn next_pow_2_size(&self) -> u32 {
        next_pow2(self.data.len() as u32).max(64)
    }
}

pub struct BrickPaletteList {
    voxels: Vec<PackedVoxelMaterial>,
    free_head: u32,
}

impl BrickPaletteList {
    pub fn new() -> Self {
        Self {
            voxels: Vec::new(),
            free_head: NULL_FREE_INDEX,
        }
    }

    // Finds a chunk with the desired palette size, if one is found that is greater, then it is
    // split
    pub fn insert(&mut self, brick_palette: BrickPalette) -> u32 {
        let palette_aligned_size = brick_palette.next_pow_2_size();
        if self.free_head != NULL_FREE_INDEX {
            let mut new_index = self.free_head;
            let mut next_free = NULL_FREE_INDEX;
            let mut last_free = NULL_FREE_INDEX;
            while new_index != NULL_FREE_INDEX {
                last_free = new_index;
                new_index = self.voxels[new_index as usize].material;
                if self.voxels[(new_index + 1) as usize].material >= palette_aligned_size {
                    next_free = self.voxels[new_index as usize].material;
                    break;
                }
            }
            if new_index == NULL_FREE_INDEX {
                return self.append(brick_palette);
            }

            todo!("Splitting free chunk");
            // let mut free_size = self.voxels[(new_index + 1) as usize].material;

            // // Split the free chunk
            // while free_size != palette_aligned_size {
            //     let new_free_size = free_size / 2;
            //     let split_index = new_index + new_free_size;
            //     self.voxels[split_index as usize] = PackedVoxelMaterial {
            //         material: next_free,
            //     };
            //     self.voxels[(split_index + 1) as usize] = PackedVoxelMaterial {
            //         material: new_free_size,
            //     };

            //     free_size = new_free_size;
            // }

            // self.free_head = self.voxels[self.free_head as usize].get_next_free_index();
            // for i in 0..brick_palette.data.len() {
            //     self.voxels[new_index as usize + i] = brick_palette.data[i as usize];
            // }

            // return new_index;
        }
        return self.append(brick_palette);
    }

    fn append(&mut self, chunk_palette: BrickPalette) -> u32 {
        let aligned_size = chunk_palette.next_pow_2_size();
        let new_index = self.voxels.len() as u32;
        self.voxels.resize(
            self.voxels.len() + aligned_size as usize,
            PackedVoxelMaterial::new([0.0, 1.0, 1.0], [0.0; 3]),
        );
        for i in 0..chunk_palette.data.len() {
            self.voxels[new_index as usize + i] = chunk_palette.data[i];
        }

        new_index
    }

    pub fn get(&self, index: u32, size: u32) -> &[PackedVoxelMaterial] {
        let index = index as usize;
        &self.voxels[index..(index + size as usize)]
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PackedVoxelMaterial {
    // 8 bit octahedron normals, 6 bits per channel rgb
    material: u32,
}

impl PackedVoxelMaterial {
    pub fn new(albedo: [f32; 3], normals: [f32; 3]) -> Self {
        let albedo = [
            (albedo[0] * 63.0) as u8,
            (albedo[1] * 63.0) as u8,
            (albedo[2] * 63.0) as u8,
        ];

        let albedo = (albedo[0] as u32) << 12 | (albedo[1] as u32) << 6 | albedo[2] as u32;
        Self { material: albedo }
    }
}
