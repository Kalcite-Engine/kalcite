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

This is an ABI foundation, not a claim that every toolkit binding is already
implemented. Platform crates can add adapters independently while preserving
the same low-level surface and GPU contract.
