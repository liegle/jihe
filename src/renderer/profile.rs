#![cfg(feature = "profile")]

use std::{f64, time::Instant};

pub struct Profiler {
    frame_counter: FrameCounter,
    gpu_profiler: wgpu_profiler::GpuProfiler,
}

impl Profiler {
    pub fn new(device: &wgpu::Device, fps_sample_size: u32) -> Self {
        Self {
            frame_counter: FrameCounter::new(fps_sample_size),
            gpu_profiler: wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap(),
        }
    }

    pub fn scope<'a>(
        &'a self,
        label: impl Into<String>,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu_profiler::Scope<'a, wgpu::CommandEncoder> {
        self.gpu_profiler.scope(label, encoder)
    }

    pub fn resolve_queries(&mut self, mut encoder: &mut wgpu::CommandEncoder) {
        self.gpu_profiler.resolve_queries(&mut encoder);
    }

    pub fn end_frame(&mut self, queue: &wgpu::Queue) {
        self.gpu_profiler.end_frame().unwrap();
        self.frame_counter.frame();
        if self.frame_counter.counted_frames == 0 {
            self.frame_counter.print();
            if let Some(results) = self
                .gpu_profiler
                .process_finished_frame(queue.get_timestamp_period())
            {
                log::info!("Gpu profile:");
                log_profiler_recursive(&results, 0);
            }
        }
    }
}

struct FrameCounter {
    sample_size: u32,
    counted_frames: u32,
    last_instant: Instant,
}

impl FrameCounter {
    fn new(sample_size: u32) -> Self {
        Self {
            sample_size,
            counted_frames: 0,
            last_instant: Instant::now(),
        }
    }

    fn frame(&mut self) {
        self.counted_frames = (self.counted_frames + 1) % self.sample_size;
    }

    fn print(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_instant;
        log::info!(
            "Fps: {:.2}",
            self.sample_size as f64 / elapsed.as_secs_f64(),
        );
        self.last_instant = now;
    }
}

fn log_profiler_recursive(results: &[wgpu_profiler::GpuTimerQueryResult], indent: usize) {
    for scope in results {
        log::info!(
            "{:>width$} {:.6}ms - {}",
            "*",
            match &scope.time {
                Some(time) => (time.end - time.start) * 1000.,
                None => f64::NAN,
            },
            scope.label,
            width = (indent + 1) * 4,
        );

        if !scope.nested_queries.is_empty() {
            log_profiler_recursive(&scope.nested_queries, indent + 1);
        }
    }
}
