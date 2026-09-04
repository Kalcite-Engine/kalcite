# Native UI surfaces and embedded GPU views

`kalcite-platform-api` exposes a fixed-capacity `SurfaceRegistry` for native
application windows and embedded game views. It has no toolkit dependency and
allocates no dynamic memory, so it is suitable for host adapters and embedded
targets alike.

Adapters map `NativeToolkit` to their host runtime: SwiftUI, GTK4, Qt6, WinUI3,
or Kotlin Compose. The adapter owns native handles and event loops; Kalcite
owns only opaque generation-checked `SurfaceId` values and layout metadata.

An application surface may embed an `EmbeddedGame` surface through
`SurfaceRegistry::embed`. The renderer receives a `GpuTarget` from that child.
Resizing the child invalidates the target generation but preserves the native
surface handle, allowing a toolkit to recreate only its swapchain/framebuffer.
Destroying the application invalidates each directly embedded child as well,
so an orphaned game view cannot retain a presentable GPU target.

For pointer and touch routing, `SurfaceRegistry::embedded_at(parent, x, y)`
performs an allocation-free hit test in the application’s logical coordinates.
If embedded views overlap, the most recently created matching view wins; an
adapter can then translate the event for that game surface without exposing its
toolkit event object to the renderer.

`route_pointer` combines that hit test with an integer coordinate transform.
It yields a `RoutedPointerEvent` in the embedded game’s logical pixel space,
preserving the pointer phase and button. A miss stays available to the native
application UI, so game input and SwiftUI/GTK/Qt/WinUI/Kotlin controls can
share one window without competing for raw events.

Keyboard focus remains a native-toolkit decision. Once an adapter has selected
a focused embedded game view, `route_key(parent, child, phase, key, modifiers)`
validates that the child is directly embedded by the application and yields a
toolkit-neutral `RoutedKeyEvent`. `NativeKeyCode` and modifier bits are opaque
adapter values; SwiftUI, GTK, Qt, WinUI, and Kotlin adapters can map their own
events to Kalcite input without sharing platform event types.

```rust
let app = surfaces.create(application)?;
let game = surfaces.create(embedded_game)?;
surfaces.embed(app, game, view_rect)?;
let target = surfaces.gpu_target(game)?;
```

Before presenting, an adapter calls `accepts_gpu_target`. This rejects a frame
prepared for a destroyed or resized surface without relying on raw pointers or
toolkit-specific lifetime rules.

`kalcite-renderer` turns its sorted draw queue into an immutable `RenderFrame`
through `Renderer::finish(target)`. The frame carries the `GpuTarget` used when
it was recorded and detaches the queue so the game can record its next frame
while a platform adapter compiles or presents the previous one. A future Metal,
Vulkan, Direct3D, OpenGL, or Skia adapter consumes this common command frame;
no renderer code needs to borrow a SwiftUI, GTK, Qt, WinUI, or Kotlin object.

`RenderFrameEncoder` makes that hand-off explicit. A backend implements
`begin_frame(target, camera)`, `draw_command(command)`, and `end_frame()` to
translate the immutable, layer-sorted commands into its own command buffer.
The adapter validates `GpuTarget` immediately before this replay, then owns
the device submission and presentation lifetime. This keeps the common
renderer API portable while allowing each backend to use its native fast path.

`kalcite-platform-headless::NativeSurfaceHost` is the executable reference for
that presentation boundary. It owns a `SurfaceRegistry`, consumes a
`RenderFrame` only after checking its target generation, and records the
accepted presentation metrics. Toolkit adapters should follow the same order:
validate, encode, then present. This keeps resize races out of adapter-specific
code and is covered by a stale-target regression test.

This is an ABI foundation, not a claim that every toolkit binding is already
implemented. Platform crates can add adapters independently while preserving
the same low-level surface and GPU contract.
