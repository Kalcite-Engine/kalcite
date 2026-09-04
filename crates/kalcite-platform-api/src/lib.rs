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

/// Native UI runtimes supported by Kalcite's platform adapters. This enum is
/// intentionally data-only: SwiftUI, GTK4, Qt6, WinUI3, and Kotlin Compose
/// remain optional host integrations rather than dependencies of the engine.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeToolkit {
    Headless,
    SwiftUi,
    Gtk4,
    Qt6,
    WinUi3,
    KotlinCompose,
    Custom,
}

/// How a native surface participates in an application. An embedded surface
/// is owned by a parent application surface and can receive a GPU frame.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRole {
    Application,
    Game,
    EmbeddedGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceDescriptor {
    pub role: SurfaceRole,
    pub width: u16,
    pub height: u16,
    /// Logical pixels per physical pixel, scaled by 100. A value of 100 is 1×.
    pub scale_x100: u16,
}

impl SurfaceDescriptor {
    pub const fn valid(self) -> bool {
        self.width > 0 && self.height > 0 && self.scale_x100 > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceId {
    slot: u16,
    generation: u16,
}

impl SurfaceId {
    pub const INVALID: Self = Self {
        slot: u16::MAX,
        generation: 0,
    };

    pub const fn slot(self) -> u16 {
        self.slot
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbeddedView {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl EmbeddedView {
    pub const fn valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Check a point in the parent application's logical coordinate space.
    /// Widening before subtraction keeps negative origins and large `u16`
    /// dimensions well-defined without signed overflow.
    pub const fn contains(self, x: i32, y: i32) -> bool {
        let left = self.x as i32;
        let top = self.y as i32;
        x >= left && y >= top && x - left < self.width as i32 && y - top < self.height as i32
    }

    /// Convert a hit-tested parent-coordinate point to logical game pixels.
    /// Multiplication is widened so a large native view cannot overflow before
    /// it is scaled down to the game's bounded target dimensions.
    pub const fn map_point(self, x: i32, y: i32, width: u16, height: u16) -> (u16, u16) {
        let local_x = (x - self.x as i32) as i64;
        let local_y = (y - self.y as i32) as i64;
        let mapped_x = local_x * width as i64 / self.width as i64;
        let mapped_y = local_y * height as i64 / self.height as i64;
        (mapped_x as u16, mapped_y as u16)
    }
}

/// The phase of a pointer event after a platform adapter has normalized it.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Move,
    Press,
    Release,
}

/// A pointer event routed into an embedded game's logical pixel space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutedPointerEvent {
    pub surface: SurfaceId,
    pub phase: PointerPhase,
    pub x: u16,
    pub y: u16,
    pub button: u8,
}

/// The phase of a keyboard event after a platform adapter has normalized it.
/// `Repeat` represents a host repeat event; adapters never need to expose
/// their native event object to the game runtime.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPhase {
    Press,
    Release,
    Repeat,
}

/// A toolkit-neutral physical/logical key identifier chosen by an adapter.
/// The value is intentionally opaque to the surface ABI: platform adapters
/// can map it to the portable game input layer without linking each other.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeKeyCode(pub u16);

/// A keyboard event routed to a focused embedded game view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutedKeyEvent {
    pub surface: SurfaceId,
    pub phase: KeyPhase,
    pub key: NativeKeyCode,
    /// Toolkit-normalized modifier bits owned by the adapter.
    pub modifiers: u16,
}

/// A generation-tagged GPU presentation target. Renderers can cache this
/// value for one frame; adapters reject it after a resize or surface destroy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuTarget {
    pub surface: SurfaceId,
    pub width: u16,
    pub height: u16,
    pub generation: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceError {
    InvalidDescriptor,
    InvalidView,
    Full,
    StaleHandle,
    InvalidEmbedding,
}

#[derive(Clone, Copy)]
struct SurfaceSlot {
    active: bool,
    generation: u16,
    target_generation: u16,
    descriptor: SurfaceDescriptor,
    parent: SurfaceId,
    view: EmbeddedView,
}

const EMPTY_DESCRIPTOR: SurfaceDescriptor = SurfaceDescriptor {
    role: SurfaceRole::Application,
    width: 0,
    height: 0,
    scale_x100: 0,
};
const EMPTY_VIEW: EmbeddedView = EmbeddedView {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};
const EMPTY_SLOT: SurfaceSlot = SurfaceSlot {
    active: false,
    generation: 0,
    target_generation: 0,
    descriptor: EMPTY_DESCRIPTOR,
    parent: SurfaceId::INVALID,
    view: EMPTY_VIEW,
};

/// Fixed-capacity lifecycle management shared by native toolkit adapters.
/// It deliberately owns no windows, handles, callbacks, or GPU objects: those
/// remain under the platform adapter's control.
pub struct SurfaceRegistry<const N: usize> {
    slots: [SurfaceSlot; N],
}

impl<const N: usize> Default for SurfaceRegistry<N> {
    fn default() -> Self {
        Self {
            slots: [EMPTY_SLOT; N],
        }
    }
}

impl<const N: usize> SurfaceRegistry<N> {
    pub fn create(&mut self, descriptor: SurfaceDescriptor) -> Result<SurfaceId, SurfaceError> {
        if !descriptor.valid() {
            return Err(SurfaceError::InvalidDescriptor);
        }
        let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| !slot.active)
        else {
            return Err(SurfaceError::Full);
        };
        slot.active = true;
        slot.generation = slot.generation.wrapping_add(1).max(1);
        slot.target_generation = 1;
        slot.descriptor = descriptor;
        slot.parent = SurfaceId::INVALID;
        slot.view = EMPTY_VIEW;
        Ok(SurfaceId {
            slot: index as u16,
            generation: slot.generation,
        })
    }

    pub fn destroy(&mut self, id: SurfaceId) -> Result<(), SurfaceError> {
        // Validate before mutating so a stale parent cannot invalidate a new
        // surface that reused its slot. An embedded game is owned by its
        // application surface, therefore it cannot outlive that parent.
        self.slot(id)?;
        for slot in &mut self.slots {
            if slot.active && slot.parent == id {
                slot.active = false;
                slot.parent = SurfaceId::INVALID;
                slot.view = EMPTY_VIEW;
            }
        }
        let slot = self.slot_mut(id)?;
        slot.active = false;
        slot.parent = SurfaceId::INVALID;
        slot.view = EMPTY_VIEW;
        Ok(())
    }

    pub fn resize(&mut self, id: SurfaceId, width: u16, height: u16) -> Result<(), SurfaceError> {
        if width == 0 || height == 0 {
            return Err(SurfaceError::InvalidDescriptor);
        }
        let slot = self.slot_mut(id)?;
        slot.descriptor.width = width;
        slot.descriptor.height = height;
        slot.target_generation = slot.target_generation.wrapping_add(1).max(1);
        Ok(())
    }

    pub fn embed(
        &mut self,
        parent: SurfaceId,
        child: SurfaceId,
        view: EmbeddedView,
    ) -> Result<(), SurfaceError> {
        if !view.valid() {
            return Err(SurfaceError::InvalidView);
        }
        let parent_slot = self.slot(parent)?;
        if parent_slot.descriptor.role != SurfaceRole::Application {
            return Err(SurfaceError::InvalidEmbedding);
        }
        let child_slot = self.slot_mut(child)?;
        if child_slot.descriptor.role != SurfaceRole::EmbeddedGame {
            return Err(SurfaceError::InvalidEmbedding);
        }
        child_slot.parent = parent;
        child_slot.view = view;
        Ok(())
    }

    pub fn gpu_target(&self, id: SurfaceId) -> Result<GpuTarget, SurfaceError> {
        let slot = self.slot(id)?;
        Ok(GpuTarget {
            surface: id,
            width: slot.descriptor.width,
            height: slot.descriptor.height,
            generation: slot.target_generation,
        })
    }

    /// Whether a frame prepared for a prior GPU target can still be presented.
    /// A resize invalidates only the target generation, not the native handle.
    pub fn accepts_gpu_target(&self, target: GpuTarget) -> bool {
        self.slot(target.surface)
            .map(|slot| {
                slot.target_generation == target.generation
                    && slot.descriptor.width == target.width
                    && slot.descriptor.height == target.height
            })
            .unwrap_or(false)
    }

    pub fn descriptor(&self, id: SurfaceId) -> Result<SurfaceDescriptor, SurfaceError> {
        Ok(self.slot(id)?.descriptor)
    }

    pub fn embedded_view(&self, id: SurfaceId) -> Result<Option<EmbeddedView>, SurfaceError> {
        let slot = self.slot(id)?;
        Ok((slot.parent != SurfaceId::INVALID).then_some(slot.view))
    }

    /// Resolve a parent-coordinate point to the topmost directly embedded
    /// game view. Native adapters can use this before translating pointer or
    /// touch input, while the renderer remains independent of toolkit events.
    pub fn embedded_at(
        &self,
        parent: SurfaceId,
        x: i32,
        y: i32,
    ) -> Result<Option<SurfaceId>, SurfaceError> {
        if self.slot(parent)?.descriptor.role != SurfaceRole::Application {
            return Err(SurfaceError::InvalidEmbedding);
        }
        for (index, slot) in self.slots.iter().enumerate().rev() {
            if slot.active && slot.parent == parent && slot.view.contains(x, y) {
                return Ok(Some(SurfaceId {
                    slot: index as u16,
                    generation: slot.generation,
                }));
            }
        }
        Ok(None)
    }

    /// Hit-test and normalize a parent-coordinate pointer event for an
    /// embedded game. A miss returns `Ok(None)`; native adapters can continue
    /// handling that event as application UI input.
    pub fn route_pointer(
        &self,
        parent: SurfaceId,
        phase: PointerPhase,
        x: i32,
        y: i32,
        button: u8,
    ) -> Result<Option<RoutedPointerEvent>, SurfaceError> {
        let Some(surface) = self.embedded_at(parent, x, y)? else {
            return Ok(None);
        };
        let slot = self.slot(surface)?;
        let (x, y) = slot
            .view
            .map_point(x, y, slot.descriptor.width, slot.descriptor.height);
        Ok(Some(RoutedPointerEvent {
            surface,
            phase,
            x,
            y,
            button,
        }))
    }

    /// Route a keyboard event to a focused directly embedded game view.
    /// Focus remains native-toolkit policy: the adapter supplies the intended
    /// child, while the registry guarantees that it belongs to `parent` and
    /// is still an active embedded game surface.
    pub fn route_key(
        &self,
        parent: SurfaceId,
        child: SurfaceId,
        phase: KeyPhase,
        key: NativeKeyCode,
        modifiers: u16,
    ) -> Result<RoutedKeyEvent, SurfaceError> {
        let parent_slot = self.slot(parent)?;
        if parent_slot.descriptor.role != SurfaceRole::Application {
            return Err(SurfaceError::InvalidEmbedding);
        }
        let child_slot = self.slot(child)?;
        if child_slot.descriptor.role != SurfaceRole::EmbeddedGame || child_slot.parent != parent {
            return Err(SurfaceError::InvalidEmbedding);
        }
        Ok(RoutedKeyEvent {
            surface: child,
            phase,
            key,
            modifiers,
        })
    }

    fn slot(&self, id: SurfaceId) -> Result<&SurfaceSlot, SurfaceError> {
        let Some(slot) = self.slots.get(id.slot as usize) else {
            return Err(SurfaceError::StaleHandle);
        };
        if !slot.active || slot.generation != id.generation {
            return Err(SurfaceError::StaleHandle);
        }
        Ok(slot)
    }

    fn slot_mut(&mut self, id: SurfaceId) -> Result<&mut SurfaceSlot, SurfaceError> {
        let Some(slot) = self.slots.get_mut(id.slot as usize) else {
            return Err(SurfaceError::StaleHandle);
        };
        if !slot.active || slot.generation != id.generation {
            return Err(SurfaceError::StaleHandle);
        }
        Ok(slot)
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;

    const APP: SurfaceDescriptor = SurfaceDescriptor {
        role: SurfaceRole::Application,
        width: 800,
        height: 600,
        scale_x100: 100,
    };
    const GAME: SurfaceDescriptor = SurfaceDescriptor {
        role: SurfaceRole::EmbeddedGame,
        width: 320,
        height: 240,
        scale_x100: 100,
    };

    #[test]
    fn embedded_game_target_tracks_resize_and_rejects_stale_handles() {
        let mut surfaces = SurfaceRegistry::<2>::default();
        let app = surfaces.create(APP).unwrap();
        let game = surfaces.create(GAME).unwrap();
        surfaces
            .embed(
                app,
                game,
                EmbeddedView {
                    x: 12,
                    y: 16,
                    width: 640,
                    height: 480,
                },
            )
            .unwrap();
        let old_target = surfaces.gpu_target(game).unwrap();
        surfaces.resize(game, 640, 480).unwrap();
        let target = surfaces.gpu_target(game).unwrap();
        assert_eq!((target.width, target.height), (640, 480));
        assert!(surfaces.accepts_gpu_target(target));
        assert!(!surfaces.accepts_gpu_target(old_target));
        assert_eq!(surfaces.embedded_view(game).unwrap().unwrap().x, 12);
        surfaces.destroy(game).unwrap();
        assert_eq!(surfaces.gpu_target(game), Err(SurfaceError::StaleHandle));
    }

    #[test]
    fn registry_never_embeds_a_regular_game_or_invalid_view() {
        let mut surfaces = SurfaceRegistry::<2>::default();
        let app = surfaces.create(APP).unwrap();
        let game = surfaces
            .create(SurfaceDescriptor {
                role: SurfaceRole::Game,
                ..GAME
            })
            .unwrap();
        assert_eq!(
            surfaces.embed(app, game, EMPTY_VIEW),
            Err(SurfaceError::InvalidView)
        );
        assert_eq!(
            surfaces.embed(
                app,
                game,
                EmbeddedView {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                }
            ),
            Err(SurfaceError::InvalidEmbedding)
        );
    }

    #[test]
    fn destroying_an_application_invalidates_its_embedded_game_target() {
        let mut surfaces = SurfaceRegistry::<2>::default();
        let app = surfaces.create(APP).unwrap();
        let game = surfaces.create(GAME).unwrap();
        surfaces
            .embed(
                app,
                game,
                EmbeddedView {
                    x: 0,
                    y: 0,
                    width: 320,
                    height: 240,
                },
            )
            .unwrap();
        let target = surfaces.gpu_target(game).unwrap();

        surfaces.destroy(app).unwrap();

        assert!(!surfaces.accepts_gpu_target(target));
        assert_eq!(surfaces.gpu_target(game), Err(SurfaceError::StaleHandle));
    }

    #[test]
    fn embedded_hit_test_routes_to_the_topmost_matching_game_view() {
        let mut surfaces = SurfaceRegistry::<3>::default();
        let app = surfaces.create(APP).unwrap();
        let first = surfaces.create(GAME).unwrap();
        let second = surfaces.create(GAME).unwrap();
        surfaces
            .embed(
                app,
                first,
                EmbeddedView {
                    x: -20,
                    y: 10,
                    width: 100,
                    height: 80,
                },
            )
            .unwrap();
        surfaces
            .embed(
                app,
                second,
                EmbeddedView {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
            )
            .unwrap();

        assert_eq!(surfaces.embedded_at(app, -1, 20), Ok(Some(first)));
        assert_eq!(surfaces.embedded_at(app, 20, 20), Ok(Some(second)));
        assert_eq!(surfaces.embedded_at(app, 100, 20), Ok(None));
    }

    #[test]
    fn pointer_routing_scales_parent_coordinates_for_the_embedded_game() {
        let mut surfaces = SurfaceRegistry::<2>::default();
        let app = surfaces.create(APP).unwrap();
        let game = surfaces.create(GAME).unwrap();
        surfaces
            .embed(
                app,
                game,
                EmbeddedView {
                    x: -20,
                    y: 10,
                    width: 640,
                    height: 480,
                },
            )
            .unwrap();

        assert_eq!(
            surfaces.route_pointer(app, PointerPhase::Press, 300, 250, 1),
            Ok(Some(RoutedPointerEvent {
                surface: game,
                phase: PointerPhase::Press,
                x: 160,
                y: 120,
                button: 1,
            }))
        );
        assert_eq!(
            surfaces.route_pointer(app, PointerPhase::Move, 700, 250, 0),
            Ok(None)
        );
    }

    #[test]
    fn keyboard_routing_requires_a_directly_embedded_game() {
        let mut surfaces = SurfaceRegistry::<3>::default();
        let app = surfaces.create(APP).unwrap();
        let game = surfaces.create(GAME).unwrap();
        let standalone = surfaces.create(GAME).unwrap();
        surfaces
            .embed(
                app,
                game,
                EmbeddedView {
                    x: 0,
                    y: 0,
                    width: 320,
                    height: 240,
                },
            )
            .unwrap();

        assert_eq!(
            surfaces.route_key(app, game, KeyPhase::Press, NativeKeyCode(42), 3),
            Ok(RoutedKeyEvent {
                surface: game,
                phase: KeyPhase::Press,
                key: NativeKeyCode(42),
                modifiers: 3,
            })
        );
        assert_eq!(
            surfaces.route_key(app, standalone, KeyPhase::Release, NativeKeyCode(42), 0),
            Err(SurfaceError::InvalidEmbedding)
        );
    }
}
