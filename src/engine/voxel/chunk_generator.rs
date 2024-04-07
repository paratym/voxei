use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};

use nalgebra::Vector3;

use super::{
    util::Morton,
    vox_constants::{
        BRICK_MORTON_LENGTH, BRICK_VOLUME, CHUNK_VOLUME, CHUNK_VOXEL_LENGTH, CHUNK_WORLD_LENGTH,
        VOXEL_WORLD_LENGTH,
    },
    vox_world::WorldChunkPos,
};

pub struct GeneratedChunk {
    pub chunk_position: WorldChunkPos,
    pub is_empty: bool,
    pub voxel_data: Vec<Option<Vector3<f32>>>,
}

impl GeneratedChunk {
    pub fn brick_data(&self, brick_morton: Morton) -> Vec<Option<Vector3<f32>>> {
        let voxel_morton_min = *brick_morton << BRICK_MORTON_LENGTH;
        let voxel_morton_max = voxel_morton_min + BRICK_VOLUME as u64;

        self.voxel_data[voxel_morton_min as usize..voxel_morton_max as usize].to_vec()
    }
}

pub struct ChunkGenerator {
    chunk_thread: Option<JoinHandle<()>>,
    chunk_req_send: Option<Sender<WorldChunkPos>>,
    chunk_gen_recv: Mutex<Receiver<GeneratedChunk>>,

    currently_generating_chunks: HashSet<WorldChunkPos>,
}

impl ChunkGenerator {
    pub fn new() -> Self {
        let (chunk_req_send, chunk_req_recv) = channel();
        let (chunk_gen_send, chunk_gen_recv) = channel();
        let chunk_thread = spawn(|| Self::thread_fn(chunk_req_recv, chunk_gen_send));
        Self {
            chunk_thread: Some(chunk_thread),
            chunk_req_send: Some(chunk_req_send),
            chunk_gen_recv: Mutex::new(chunk_gen_recv),
            currently_generating_chunks: HashSet::new(),
        }
    }

    pub fn generate_chunk(&mut self, chunk_pos: WorldChunkPos) {
        if self.currently_generating_chunks.contains(&chunk_pos) {
            return;
        }

        self.chunk_req_send
            .as_ref()
            .unwrap()
            .send(chunk_pos)
            .unwrap();
        self.currently_generating_chunks.insert(chunk_pos);
    }

    pub fn collect_generated_chunks(&mut self) -> Vec<GeneratedChunk> {
        let reciever = self.chunk_gen_recv.get_mut().unwrap();

        let mut chunks = Vec::new();
        while let Ok(chunk) = reciever.try_recv() {
            self.currently_generating_chunks
                .remove(&chunk.chunk_position);
            chunks.push(chunk);
        }
        chunks
    }

    fn thread_fn(chunk_req_recv: Receiver<WorldChunkPos>, chunk_gen_send: Sender<GeneratedChunk>) {
        while let Ok(chunk_pos) = chunk_req_recv.recv() {
            let chunk_voxel_min = chunk_pos.vector * CHUNK_VOXEL_LENGTH as i32;

            let mut data = vec![None; CHUNK_VOLUME * BRICK_VOLUME];
            let mut is_empty = true;
            for x in 0..CHUNK_VOXEL_LENGTH {
                for y in 0..CHUNK_VOXEL_LENGTH {
                    for z in 0..CHUNK_VOXEL_LENGTH {
                        let world_vox_pos = Vector3::new(
                            chunk_voxel_min.x + x as i32,
                            chunk_voxel_min.y + y as i32,
                            chunk_voxel_min.z + z as i32,
                        );
                        if world_vox_pos.y == 0 {
                            let morton = Morton::encode(Vector3::new(x as u32, y as u32, z as u32));
                            data[*morton as usize] = Some(Vector3::new(0.6, 0.2, 0.7));
                            is_empty = false;
                        }
                    }
                }
            }

            // Have this thread just do all the generating for now.
            chunk_gen_send
                .send(GeneratedChunk {
                    is_empty,
                    chunk_position: chunk_pos,
                    voxel_data: data,
                })
                .expect("Failed to send generated chunk.");
        }
    }
}

impl Drop for ChunkGenerator {
    fn drop(&mut self) {
        drop(
            self.chunk_req_send
                .take()
                .expect("Failed to take chunk request sender."),
        );
        self.chunk_thread
            .take()
            .unwrap()
            .join()
            .expect("Failed to join chunk generation thread.");
    }
}
