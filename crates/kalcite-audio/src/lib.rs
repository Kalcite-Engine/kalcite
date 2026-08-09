#[derive(Clone, Copy)]
pub enum Command {
    Tone { hz: u16, ms: u16, volume: u8 },
    Stop,
}
pub trait Backend {
    fn submit(&mut self, c: Command);
}
