use std::time::Instant;

pub struct FrametimeHistogram {
    samples: Vec<f32>,
    index: usize,
    last_sample_time: Instant,
}

impl FrametimeHistogram {
    pub fn new(size: usize) -> Self {
        Self {
            samples: vec![0.0; size],
            index: 0,
            last_sample_time: Instant::now(),
        }
    }

    pub fn push(&mut self, frametime: f32) {
        if self.last_sample_time.elapsed().as_millis() >= 100 {
            self.samples[self.index] = frametime;
            self.index = (self.index + 1) % self.samples.len();
            self.last_sample_time = Instant::now();
        }
    }

    pub fn average(&self) -> f32 {
        self.samples.iter().sum::<f32>() / self.samples.len() as f32
    }
}
