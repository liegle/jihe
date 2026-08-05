pub(super) struct Config {
    pub(super) render_per_sec: u64,
    pub(super) resize_per_sec: u64,
    pub(super) move_speed: f32,
    pub(super) zoom_factor: f32,
}

impl Config {
    pub(super) fn new() -> Self {
        Self {
            render_per_sec: 60,
            resize_per_sec: 60,
            move_speed: 30.,
            zoom_factor: 0.2,
        }
    }
}
