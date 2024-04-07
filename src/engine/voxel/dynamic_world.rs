use nalgebra::{SimdPartialOrd, Vector3};

use crate::{engine::voxel::vox_constants::CHUNK_VOLUME, settings::Settings};

use super::{
    chunk_generator::GeneratedChunk,
    util::Morton,
    vox_constants::{BRICK_AREA, BRICK_VOLUME, SUPER_CHUNK_VOLUME},
    vox_world::{ChunkRadius, DynChunkPos, WorldChunkPos},
};

/// Our voxel world representation for rendering and is more easily editable due to the flat array
/// structure for each brick and masks for each hierarchy level.
pub struct DynVoxelWorld {
    super_chunk_grid_mask: GridMask,
    chunk_occupancy_mask: GridMask,
    brick_indices_grid: BrickIndexGrid,
    brick_data: BrickDataList,

    brick_changes: Vec<BrickChange>,

    chunk_render_distance: ChunkRadius,
}

impl DynVoxelWorld {
    pub fn new(settings: &Settings) -> Self {
        let chunk_render_volume = settings.chunk_render_distance.pow2_volume();
        println!("Chunk render volume: {}", chunk_render_volume);
        let super_chunk_render_volume =
            (chunk_render_volume as f32 / SUPER_CHUNK_VOLUME as f32).ceil() as u64;
        let brick_render_volume = chunk_render_volume * BRICK_VOLUME as u64;
        println!("Brick render volume: {}", brick_render_volume);

        Self {
            super_chunk_grid_mask: GridMask::new(super_chunk_render_volume as usize),
            chunk_occupancy_mask: GridMask::new(chunk_render_volume as usize),
            brick_indices_grid: BrickIndexGrid::new(brick_render_volume as usize),
            brick_data: BrickDataList::new(),

            brick_changes: Vec::new(),

            chunk_render_distance: settings.chunk_render_distance,
        }
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
            println!("Chunk at {:?} is empty.", local_chunk_pos);
        } else {
            self.chunk_occupancy_mask
                .set_status(morton, SpatialStatus::Loaded);
            let local_brick_min = local_chunk_pos.to_dyn_brick_pos();
            let local_brick_min_morton = *local_brick_min.morton();
            for brick_morton in 0..CHUNK_VOLUME {
                let generated_voxels = chunk.brick_data(Morton::new(brick_morton as u64));
                let is_empty = generated_voxels.iter().all(|voxel| voxel.is_none());

                let dyn_brick_morton = local_brick_min_morton + brick_morton as u64;
                if is_empty {
                    self.set_brick(dyn_brick_morton, None);
                } else {
                    let brick_data = BrickData::from_voxel_array(generated_voxels);
                    self.set_brick(dyn_brick_morton, Some(brick_data));
                }
            }
        }
    }

    pub fn set_chunk_loading(&mut self, local_chunk_pos: DynChunkPos) {
        let morton = local_chunk_pos.morton();
        self.chunk_occupancy_mask
            .set_status(morton, SpatialStatus::Loading);
    }

    pub fn set_brick(&mut self, morton: u64, brick: Option<BrickData>) {
        let brick_index = if let Some(brick) = brick {
            let index = self.brick_data.insert(brick);
            BrickIndex::new_loaded(index)
        } else {
            BrickIndex::new_loaded_empty()
        };
        self.brick_changes.push(BrickChange {
            brick_morton: Morton::new(morton),
        });
        self.brick_indices_grid.0[morton as usize] = brick_index;
    }

    pub fn collect_brick_changes(&mut self) -> Vec<BrickChange> {
        std::mem::replace(&mut self.brick_changes, Vec::new())
    }

    pub fn chunk_occupancy_grid(&self) -> &GridMask {
        &self.chunk_occupancy_mask
    }

    pub fn brick_indices_grid(&self) -> &BrickIndexGrid {
        &self.brick_indices_grid
    }

    pub fn chunk_render_distance(&self) -> ChunkRadius {
        self.chunk_render_distance
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
}

const NULL_FREE_INDEX: u32 = 0x7FFFFFFF;

impl BrickDataList {
    pub fn new() -> Self {
        Self {
            free_head: NULL_FREE_INDEX,
            data: Vec::new(),
        }
    }

    /// Returns the brick list index for the inserted brick data.
    pub fn insert(&mut self, brick_data: BrickData) -> u32 {
        if self.free_head != NULL_FREE_INDEX {
            let new_index = self.free_head;
            self.free_head = self.data[self.free_head as usize].next_free();
            self.data[new_index as usize] = brick_data;

            return new_index;
        } else {
            self.data.push(brick_data);
            return self.data.len() as u32 - 1;
        }
    }

    pub fn get(&self, index: u32) -> &BrickData {
        &self.data[index as usize]
    }
}

pub struct BrickData {
    // last bit determines if this is a free brick data, then the index points to the next free
    // brick.
    material_index: u32,
    voxel_mask: [u8; BRICK_AREA],
}

impl BrickData {
    pub fn new_free(next_free: u32) -> Self {
        Self {
            material_index: next_free | 0x8000_0000,
            voxel_mask: [0; BRICK_AREA],
        }
    }

    pub fn set_free(&mut self, next_free: u32) {
        self.material_index = next_free | 0x8000_0000;
    }

    pub fn next_free(&self) -> u32 {
        self.material_index & 0x7FFF_FFFF
    }

    pub fn from_voxel_array(voxel_data: Vec<Option<Vector3<f32>>>) -> Self {
        let mut voxel_mask = [0; BRICK_AREA];
        for i in 0..BRICK_VOLUME {
            let voxel = &voxel_data[i];
            if voxel.is_some() {
                voxel_mask[i >> 3] = 1 << (i & 0b111);
            }
        }

        Self {
            // TODO - materials
            material_index: 0,
            voxel_mask,
        }
    }
}

pub struct BrickChange {
    pub brick_morton: Morton,
}
