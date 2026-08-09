use kalcite_platform_api::{Buttons, Platform};
pub struct Headless<const N: usize> {
    pub width: u16,
    pub height: u16,
    pub now: u32,
    pub input: Buttons,
    pub frame: [u16; N],
    pub presents: u32,
}
impl<const N: usize> Headless<N> {
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            now: 0,
            input: Buttons(0),
            frame: [0; N],
            presents: 0,
        }
    }
}
impl<const N: usize> Platform for Headless<N> {
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
    fn ticks_ms(&self) -> u32 {
        self.now
    }
    fn buttons(&mut self) -> Buttons {
        self.input
    }
    fn present(&mut self, p: &[u16]) {
        let n = p.len().min(N);
        self.frame[..n].copy_from_slice(&p[..n]);
        self.presents += 1
    }
}
