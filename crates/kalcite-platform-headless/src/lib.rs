use kalcite_platform_api::{Buttons, GpuTarget, Platform, SurfaceRegistry};
use kalcite_renderer::RenderFrame;

/// Outcome of submitting a renderer frame to a native-surface adapter.
///
/// The target is checked immediately before presentation, keeping a resized
/// surface from ever accepting commands recorded for its former swapchain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePresentError {
    StaleTarget,
}

/// A no-window reference adapter for the native UI/GPU contract.
///
/// It does not own a toolkit handle or a GPU device. Instead it demonstrates
/// the required lifecycle: toolkit adapters own their resources, while this
/// common host validates a generation-tagged `RenderFrame` just before they
/// encode or present it.
pub struct NativeSurfaceHost<const SURFACES: usize> {
    pub surfaces: SurfaceRegistry<SURFACES>,
    presented_frames: u32,
    last_target: Option<GpuTarget>,
    last_draw_calls: u32,
}

impl<const SURFACES: usize> Default for NativeSurfaceHost<SURFACES> {
    fn default() -> Self {
        Self {
            surfaces: SurfaceRegistry::default(),
            presented_frames: 0,
            last_target: None,
            last_draw_calls: 0,
        }
    }
}

impl<const SURFACES: usize> NativeSurfaceHost<SURFACES> {
    /// Validate and consume a frame at the presentation boundary.
    pub fn present(&mut self, frame: RenderFrame) -> Result<(), FramePresentError> {
        if !self.surfaces.accepts_gpu_target(frame.target()) {
            return Err(FramePresentError::StaleTarget);
        }
        self.presented_frames = self.presented_frames.saturating_add(1);
        self.last_target = Some(frame.target());
        self.last_draw_calls = frame.draw_calls();
        Ok(())
    }

    pub const fn presented_frames(&self) -> u32 {
        self.presented_frames
    }

    pub const fn last_target(&self) -> Option<GpuTarget> {
        self.last_target
    }

    pub const fn last_draw_calls(&self) -> u32 {
        self.last_draw_calls
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use kalcite_platform_api::{SurfaceDescriptor, SurfaceRole};
    use kalcite_renderer::{Renderer, Sprite};

    const EMBEDDED_GAME: SurfaceDescriptor = SurfaceDescriptor {
        role: SurfaceRole::EmbeddedGame,
        width: 320,
        height: 240,
        scale_x100: 100,
    };

    #[test]
    fn presentation_accepts_only_the_current_gpu_target() {
        let mut host = NativeSurfaceHost::<1>::default();
        let surface = host.surfaces.create(EMBEDDED_GAME).unwrap();
        let target = host.surfaces.gpu_target(surface).unwrap();

        let mut renderer = Renderer::default();
        renderer.push(Sprite {
            asset: 1,
            x: 0,
            y: 0,
            layer: 0,
        });
        host.present(renderer.finish(target)).unwrap();
        assert_eq!(host.presented_frames(), 1);
        assert_eq!(host.last_target(), Some(target));
        assert_eq!(host.last_draw_calls(), 1);

        host.surfaces.resize(surface, 640, 480).unwrap();
        assert_eq!(
            host.present(Renderer::default().finish(target)),
            Err(FramePresentError::StaleTarget)
        );
        assert_eq!(host.presented_frames(), 1);
    }
}
