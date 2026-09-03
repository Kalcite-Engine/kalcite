#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub asset: u64,
    pub x: i16,
    pub y: i16,
    pub layer: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteRegion {
    pub asset: u64,
    pub source_x: u16,
    pub source_y: u16,
    pub width: u16,
    pub height: u16,
    pub x: i16,
    pub y: i16,
    pub layer: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tilemap {
    pub map: u64,
    pub tileset: u64,
    pub tile_w: u16,
    pub tile_h: u16,
    pub x: i16,
    pub y: i16,
    pub layer: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawCommand {
    Sprite(Sprite),
    SpriteRegion(SpriteRegion),
    Tilemap(Tilemap),
}

/// An immutable render submission prepared for one generation of a native GPU
/// surface. Backends must check the target generation before presenting; this
/// prevents a frame prepared before resize from reaching a replaced swapchain.
#[derive(Debug, PartialEq, Eq)]
pub struct RenderFrame {
    target: GpuTarget,
    camera: Camera,
    commands: Vec<DrawCommand>,
}

impl RenderFrame {
    pub fn target(&self) -> GpuTarget {
        self.target
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn draw_calls(&self) -> u32 {
        self.commands.len() as u32
    }
}
#[derive(Default)]
pub struct Renderer {
    pub camera: Camera,
    queue: Vec<DrawCommand>,
}
impl Renderer {
    pub fn push(&mut self, sprite: Sprite) {
        self.queue.push(DrawCommand::Sprite(sprite));
    }
    pub fn push_tilemap(&mut self, map: Tilemap) {
        self.queue.push(DrawCommand::Tilemap(map));
    }
    pub fn push_region(&mut self, region: SpriteRegion) {
        self.queue.push(DrawCommand::SpriteRegion(region));
    }
    pub fn set_camera(&mut self, x: i32, y: i32) {
        self.camera = Camera { x, y };
    }
    pub fn sorted(&mut self) -> &[DrawCommand] {
        self.queue.sort_by_key(|c| match c {
            DrawCommand::Sprite(v) => v.layer,
            DrawCommand::SpriteRegion(v) => v.layer,
            DrawCommand::Tilemap(v) => v.layer,
        });
        &self.queue
    }
    pub fn world_to_screen(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.camera.x, y - self.camera.y)
    }
    pub fn clear(&mut self) {
        self.queue.clear();
    }
    pub fn draw_calls(&self) -> u32 {
        self.queue.len() as u32
    }

    /// Sort and detach the current command queue for a GPU backend. The
    /// renderer becomes ready to record the next frame immediately afterwards.
    pub fn finish(&mut self, target: GpuTarget) -> RenderFrame {
        self.queue.sort_by_key(|command| match command {
            DrawCommand::Sprite(sprite) => sprite.layer,
            DrawCommand::SpriteRegion(region) => region.layer,
            DrawCommand::Tilemap(tilemap) => tilemap.layer,
        });
        RenderFrame {
            target,
            camera: self.camera,
            commands: core::mem::take(&mut self.queue),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layer_order() {
        let mut r = Renderer::default();
        r.push(Sprite {
            asset: 1,
            x: 0,
            y: 0,
            layer: 2,
        });
        r.push(Sprite {
            asset: 2,
            x: 0,
            y: 0,
            layer: -1,
        });
        assert!(matches!(
            r.sorted()[0],
            DrawCommand::Sprite(Sprite { asset: 2, .. })
        ));
    }

    #[test]
    fn equal_layers_preserve_submission_order() {
        let mut renderer = Renderer::default();
        renderer.push(Sprite {
            asset: 1,
            x: 0,
            y: 0,
            layer: 2,
        });
        renderer.push_region(SpriteRegion {
            asset: 2,
            source_x: 0,
            source_y: 0,
            width: 8,
            height: 8,
            x: 0,
            y: 0,
            layer: 2,
        });
        assert!(matches!(
            renderer.sorted(),
            [
                DrawCommand::Sprite(Sprite { asset: 1, .. }),
                DrawCommand::SpriteRegion(SpriteRegion { asset: 2, .. })
            ]
        ));
    }

    #[test]
    fn render_frame_is_sorted_and_detaches_the_recording_queue() {
        let mut renderer = Renderer::default();
        renderer.push(Sprite {
            asset: 2,
            x: 0,
            y: 0,
            layer: 2,
        });
        renderer.push(Sprite {
            asset: 1,
            x: 0,
            y: 0,
            layer: 1,
        });
        let target = GpuTarget {
            surface: kalcite_platform_api::SurfaceId::INVALID,
            width: 320,
            height: 240,
            generation: 7,
        };
        let frame = renderer.finish(target);
        assert_eq!(renderer.draw_calls(), 0);
        assert_eq!(frame.target(), target);
        assert!(matches!(
            frame.commands(),
            [
                DrawCommand::Sprite(Sprite { asset: 1, .. }),
                DrawCommand::Sprite(Sprite { asset: 2, .. })
            ]
        ));
    }
}
use kalcite_platform_api::GpuTarget;
