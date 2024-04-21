use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex, RwLock,
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
    vox_world::{ChunkRadius, WorldChunkPos},
};

pub struct GeneratedChunk {
    pub chunk_position: WorldChunkPos,
    pub is_empty: bool,
    // If voxel_data is none then it was not generated cause out of bounds..
    pub voxel_data: Option<Vec<Option<Vector3<f32>>>>,
}

impl GeneratedChunk {
    pub fn brick_data(&self, brick_morton: Morton) -> Vec<Option<Vector3<f32>>> {
        let voxel_morton_min = *brick_morton << BRICK_MORTON_LENGTH;
        let voxel_morton_max = voxel_morton_min + BRICK_VOLUME as u64;

        self.voxel_data.as_ref().unwrap()[voxel_morton_min as usize..voxel_morton_max as usize]
            .to_vec()
    }
}

pub struct ChunkGenerator {
    chunk_thread: Option<JoinHandle<()>>,
    chunk_req_send: Option<Sender<WorldChunkPos>>,
    chunk_gen_recv: Mutex<Receiver<GeneratedChunk>>,

    dyn_world_chunk_bounds: Arc<RwLock<(Vector3<i32>, Vector3<i32>)>>,

    currently_generating_chunks: HashSet<WorldChunkPos>,
    is_running: Arc<AtomicBool>,
}

pub struct ChunkGeneratorThread {
    chunk_req_recv: Receiver<WorldChunkPos>,
    chunk_gen_send: Sender<GeneratedChunk>,

    dyn_world_chunk_bounds: Arc<RwLock<(Vector3<i32>, Vector3<i32>)>>,
    is_running: Arc<AtomicBool>,
}

impl ChunkGenerator {
    pub fn new() -> Self {
        let (chunk_req_send, chunk_req_recv) = channel();
        let (chunk_gen_send, chunk_gen_recv) = channel();
        let dyn_world_chunk_bounds =
            Arc::new(RwLock::new((Vector3::new(0, 0, 0), Vector3::new(0, 0, 0))));
        let is_running = Arc::new(AtomicBool::new(true));
        let bc = dyn_world_chunk_bounds.clone();
        let is = is_running.clone();
        let chunk_thread = spawn(|| {
            Self::thread_fn(ChunkGeneratorThread {
                chunk_req_recv,
                chunk_gen_send,
                dyn_world_chunk_bounds: bc,
                is_running: is,
            })
        });
        Self {
            chunk_thread: Some(chunk_thread),
            chunk_req_send: Some(chunk_req_send),
            chunk_gen_recv: Mutex::new(chunk_gen_recv),
            currently_generating_chunks: HashSet::new(),

            dyn_world_chunk_bounds,
            is_running,
        }
    }

    // Update bounds so we skip generating chunks that are no longer in bounds of the dyn world.
    pub fn update_bounds(&mut self, chunk_center: WorldChunkPos, render_distance: ChunkRadius) {
        let hl = render_distance.pow2_half_side_length() as i32;
        let hl = Vector3::new(hl, hl, hl);
        let mut dyn_world_chunk_bounds = self.dyn_world_chunk_bounds.write().unwrap();
        dyn_world_chunk_bounds.0 = chunk_center.vector - hl;
        dyn_world_chunk_bounds.1 = chunk_center.vector + hl - Vector3::new(1, 1, 1);
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

            // The chunk was not discarded during generation so collect it
            if chunk.voxel_data.is_some() {
                chunks.push(chunk);
            }
        }
        chunks
    }

    fn thread_fn(th: ChunkGeneratorThread) {
        while let Ok(chunk_pos) = th.chunk_req_recv.recv() {
            if !th.is_running.load(Ordering::SeqCst) {
                break;
            }

            let bounds = th.dyn_world_chunk_bounds.read().unwrap().clone();
            if chunk_pos.vector.x < bounds.0.x
                || chunk_pos.vector.y < bounds.0.y
                || chunk_pos.vector.z < bounds.0.z
                || chunk_pos.vector.x >= bounds.1.x
                || chunk_pos.vector.y >= bounds.1.y
                || chunk_pos.vector.z >= bounds.1.z
            {
                th.chunk_gen_send
                    .send(GeneratedChunk {
                        is_empty: false,
                        chunk_position: chunk_pos,
                        voxel_data: None,
                    })
                    .expect("Failed to send generated chunk.");
                continue;
            }

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

                        let height = (world_vox_pos.x as f32 / 32.0).sin() * 10.0
                            + (world_vox_pos.z as f32 / 13.0
                                + (world_vox_pos.z as f32 / 16.0).sin())
                            .cos()
                                * 15.0;
                        let diff = height - world_vox_pos.y as f32;
                        if diff <= 3.0 && diff >= 0.0 {
                            let morton = Morton::encode(Vector3::new(x as u32, y as u32, z as u32));
                            let random_y = rand::random::<f32>() * 0.1;
                            let random_x = rand::random::<f32>() * 0.075;
                            data[*morton as usize] =
                                Some(Vector3::new(random_x, 0.5 + random_y, 0.0));
                            is_empty = false;
                        }
                    }
                }
            }

            // Have this thread just do all the generating for now.
            th.chunk_gen_send
                .send(GeneratedChunk {
                    is_empty,
                    chunk_position: chunk_pos,
                    voxel_data: Some(data),
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
        self.is_running.store(false, Ordering::SeqCst);
        self.chunk_thread
            .take()
            .unwrap()
            .join()
            .expect("Failed to join chunk generation thread.");
    }
}
