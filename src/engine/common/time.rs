use std::time::{Duration, Instant};

use voxei_macros::Resource;

use crate::engine::resource::ResMut;

#[derive(Resource)]
pub struct Time {
    delta_time: Duration,
    time: Instant,
    last_time: Instant,
}

impl Time {
    pub fn new() -> Self {
        let time = Instant::now();

        Self {
            delta_time: Duration::from_secs(0),
            time,
            last_time: time,
        }
    }

    pub fn update(mut time: ResMut<Time>) {
        time.last_time = time.time;
        time.time = Instant::now();
        time.delta_time = time.time.duration_since(time.last_time);
        println!(
            "Delta in ms: {}",
            time.delta_time.as_micros() as f32 / 1000.0
        );
        println!(
            "FPS: {}",
            1.0 / (time.delta_time.as_micros() as f32 / 1000000.0)
        );
    }

    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    pub fn time(&self) -> Instant {
        self.time
    }

    pub fn last_time(&self) -> Instant {
        self.last_time
    }
}
