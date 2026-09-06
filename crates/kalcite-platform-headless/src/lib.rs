use kalcite_platform_api::{Buttons, GpuTarget, Platform, SurfaceRegistry};
use kalcite_renderer::{RenderFrame, RenderFrameEncoder};

/// Outcome of submitting a renderer frame to a native-surface adapter.
///
/// The target is checked immediately before presentation, keeping a resized
/// surface from ever accepting commands recorded for its former swapchain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePresentError {
    StaleTarget,
}

/// Failure while encoding a generation-validated renderer frame for a native
/// adapter. Encoding errors do not count as a presented frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEncodeError<E> {
    StaleTarget,
    Encoder(E),
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
    fn validate_frame(&self, frame: &RenderFrame) -> Result<(), FramePresentError> {
        if !self.surfaces.accepts_gpu_target(frame.target()) {
            return Err(FramePresentError::StaleTarget);
        }
        Ok(())
    }

    fn record_presented(&mut self, frame: &RenderFrame) {
        self.presented_frames = self.presented_frames.saturating_add(1);
        self.last_target = Some(frame.target());
        self.last_draw_calls = frame.draw_calls();
    }

    /// Validate and consume a frame at the presentation boundary.
    pub fn present(&mut self, frame: RenderFrame) -> Result<(), FramePresentError> {
        self.validate_frame(&frame)?;
        self.record_presented(&frame);
        Ok(())
    }

    /// Validate a frame immediately before replaying it into a native GPU
    /// encoder, then record presentation only after the encoder succeeds.
    /// This gives Metal, Vulkan, Direct3D, OpenGL, and Skia adapters one
    /// toolkit-independent lifecycle without giving the engine a device.
    pub fn encode_and_present<E: RenderFrameEncoder>(
        &mut self,
        frame: &RenderFrame,
        encoder: &mut E,
    ) -> Result<(), FrameEncodeError<E::Error>> {
        self.validate_frame(frame)
            .map_err(|_| FrameEncodeError::StaleTarget)?;
        frame.encode(encoder).map_err(FrameEncodeError::Encoder)?;
        self.record_presented(frame);
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
    use kalcite_renderer::{Camera, DrawCommand, Renderer, Sprite};

    const EMBEDDED_GAME: SurfaceDescriptor = SurfaceDescriptor {
        role: SurfaceRole::EmbeddedGame,
        width: 320,
        height: 240,
        scale_x100: 100,
    };

    #[derive(Default)]
    struct CountingEncoder {
        began: u32,
        commands: u32,
        ended: u32,
    }

    impl RenderFrameEncoder for CountingEncoder {
        type Error = ();

        fn begin_frame(&mut self, _: GpuTarget, _: Camera) -> Result<(), Self::Error> {
            self.began += 1;
            Ok(())
        }

        fn draw_command(&mut self, _: DrawCommand) -> Result<(), Self::Error> {
            self.commands += 1;
            Ok(())
        }

        fn end_frame(&mut self) -> Result<(), Self::Error> {
            self.ended += 1;
            Ok(())
        }
    }

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

    #[test]
    fn encoding_presents_only_after_a_current_frame_finishes() {
        let mut host = NativeSurfaceHost::<1>::default();
        let surface = host.surfaces.create(EMBEDDED_GAME).unwrap();
        let target = host.surfaces.gpu_target(surface).unwrap();
        let mut renderer = Renderer::default();
        renderer.push(Sprite {
            asset: 7,
            x: 12,
            y: 18,
            layer: 0,
        });
        let frame = renderer.finish(target);
        let mut encoder = CountingEncoder::default();

        host.encode_and_present(&frame, &mut encoder).unwrap();

        assert_eq!((encoder.began, encoder.commands, encoder.ended), (1, 1, 1));
        assert_eq!(host.presented_frames(), 1);
        host.surfaces.resize(surface, 640, 480).unwrap();
        assert_eq!(
            host.encode_and_present(&frame, &mut encoder),
            Err(FrameEncodeError::StaleTarget)
        );
        assert_eq!(host.presented_frames(), 1);
        assert_eq!((encoder.began, encoder.commands, encoder.ended), (1, 1, 1));
    }
}
