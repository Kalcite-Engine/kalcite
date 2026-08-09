#![no_std]
#[derive(Clone, Copy, Default)]
pub struct Buttons(pub u32);
impl Buttons {
    pub const LEFT: u32 = 1 << 0;
    pub const RIGHT: u32 = 1 << 1;
    pub const UP: u32 = 1 << 2;
    pub const DOWN: u32 = 1 << 3;
    pub const OK: u32 = 1 << 4;
    pub fn held(self, b: u32) -> bool {
        self.0 & b != 0
    }
}
pub trait Platform {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn ticks_ms(&self) -> u32;
    fn buttons(&mut self) -> Buttons;
    fn present(&mut self, pixels: &[u16]);
}
