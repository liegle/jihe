use std::time::Instant;

pub struct FrameCounter {
    interval_frames: u32,
    counted_frames: u32,
    last_instant: Option<Instant>,
}

impl FrameCounter {
    pub fn new(interval_frames: u32) -> Self {
        Self {
            interval_frames,
            counted_frames: 0,
            last_instant: None,
        }
    }

    pub fn frame(&mut self) {
        let Some(last_instant) = self.last_instant else {
            self.last_instant = Some(Instant::now());
            return;
        };
        self.counted_frames += 1;
        if self.counted_frames == self.interval_frames {
            let now = Instant::now();
            let elapsed = now - last_instant;
            log::info!(
                "Fps: {:.2}, average frame time: {:.2} ms",
                self.interval_frames as f64 / elapsed.as_secs_f64(),
                elapsed.as_millis() as f64 / self.interval_frames as f64
            );
            self.counted_frames = 0;
            self.last_instant = Some(now);
        }
    }
}
