#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Frame {
    pub frame_us: u32,
    pub update_us: u32,
    pub render_us: u32,
    pub physics_us: u32,
    pub draw_calls: u32,
    pub dirty_pixels: u32,
    pub dirty_regions: u32,
    pub sprites: u32,
    pub tiles: u32,
    pub collision_queries: u32,
    pub pool_used: u32,
    pub static_ram: u32,
}
#[derive(Default)]
pub struct Profiler {
    current: Frame,
    peak: Frame,
    frames: u64,
    total_us: u64,
}
impl Profiler {
    pub fn begin(&mut self, static_ram: u32) {
        self.current = Frame {
            static_ram,
            ..Frame::default()
        }
    }
    pub fn draw(&mut self, pixels: u32) {
        self.current.draw_calls = self.current.draw_calls.saturating_add(1);
        self.current.dirty_pixels = self.current.dirty_pixels.saturating_add(pixels)
    }
    pub fn engine(
        &mut self,
        update_us: u32,
        render_us: u32,
        physics_us: u32,
        dirty_regions: u32,
        sprites: u32,
        tiles: u32,
        collision_queries: u32,
    ) {
        self.current.update_us = update_us;
        self.current.render_us = render_us;
        self.current.physics_us = physics_us;
        self.current.dirty_regions = dirty_regions;
        self.current.sprites = sprites;
        self.current.tiles = tiles;
        self.current.collision_queries = collision_queries;
    }
    pub fn pool_used(&mut self, n: u32) {
        self.current.pool_used = n
    }
    pub fn end(&mut self, frame_us: u32) -> Frame {
        self.current.frame_us = frame_us;
        self.frames += 1;
        self.total_us += frame_us as u64;
        self.peak.frame_us = self.peak.frame_us.max(frame_us);
        self.peak.draw_calls = self.peak.draw_calls.max(self.current.draw_calls);
        self.peak.dirty_pixels = self.peak.dirty_pixels.max(self.current.dirty_pixels);
        self.peak.update_us = self.peak.update_us.max(self.current.update_us);
        self.peak.render_us = self.peak.render_us.max(self.current.render_us);
        self.peak.physics_us = self.peak.physics_us.max(self.current.physics_us);
        self.peak.dirty_regions = self.peak.dirty_regions.max(self.current.dirty_regions);
        self.peak.sprites = self.peak.sprites.max(self.current.sprites);
        self.peak.tiles = self.peak.tiles.max(self.current.tiles);
        self.peak.collision_queries = self
            .peak
            .collision_queries
            .max(self.current.collision_queries);
        self.peak.pool_used = self.peak.pool_used.max(self.current.pool_used);
        self.peak.static_ram = self.peak.static_ram.max(self.current.static_ram);
        self.current
    }
    pub fn peak(&self) -> Frame {
        self.peak
    }
    pub fn average_frame_us(&self) -> u32 {
        if self.frames == 0 {
            0
        } else {
            (self.total_us / self.frames) as u32
        }
    }
}
