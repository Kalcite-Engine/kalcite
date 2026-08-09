# Engine readiness roadmap

Kalcite already has a native compiled language, typed locals, HIR/MIR, desktop and NumWorks backends, fixed pools/handles, persistent storage, bundled libraries, RGB565 rendering, hardware input, a hardware qualification app and native Rust/ASM escape hatches.

The remaining work to make it a broadly usable game engine is grouped below.

## P0 - game-production baseline

- Scene compiler: compile `.kscn` into static node/component tables, resolve `@node`, `@export`, signals and autoloads at build time.
- Real asset pipeline: PNG/spritesheets/fonts/tilemaps/audio metadata -> target-optimized binary assets with deduplication and compression.
- Sprite/tile renderer: clipped blits, transparency/keying, palettes, tilemaps, cameras and layers, with NumWorks dirty-region integration.
- Collision/physics-lite: AABB, circles, tile collision, ray/overlap queries and deterministic fixed-point movement.
- Input actions: named actions, just-pressed/released state, remapping on desktop, compact bitsets on embedded targets.
- Audio abstraction: desktop implementation first; target-specific capabilities with graceful unsupported states.
- Save API v2: typed blobs/MessagePack objects, schema versions, migration helpers, atomic-ish replace strategy and corruption checks.
- Error diagnostics: filenames, line/column spans, source excerpts, cross-file type errors and actionable compiler messages.

## P1 - developer experience

- Language server (LSP): completion, hover, go-to-definition, rename, diagnostics and cross-file symbols for VS Code/Zed.
- Inspector/scene editor: edit exported properties and attach scripts without hand-editing scene files.
- Hot reload on desktop for scripts/assets where ABI layout is unchanged.
- Debugger/profiler: frame timing, draw-call count, dirty pixels, pool occupancy, static-RAM budget and per-system timings.
- Unit/integration test syntax in KLC and headless CI runner.
- Package manager/lockfile for external KLC/Rust libraries with target capability declarations.

## P2 - compiler/runtime power

- Full semantic type checker and borrow/alias rules appropriate for fixed pools/handles.
- SSA-like MIR passes: constant propagation, dead-store elimination, inlining, devirtualization and range-based integer narrowing.
- Whole-program memory planner: automatic packing/reordering, pool sizing suggestions and stack high-water estimates.
- Asset/code link-time stripping driven by actual symbol use.
- Generics/templates with monomorphization and zero runtime metadata.
- Interfaces/traits with static dispatch by default and explicitly-costed dynamic dispatch.
- Deterministic coroutines/state machines lowered to structs + switches, not a VM.

## P3 - more platforms

- Native Linux/macOS/Windows packaging around the shared desktop backend.
- WebAssembly backend.
- Optional handheld/embedded backends that implement the same platform contracts.

NumWorks remains the strict design target: features should have a bounded-memory lowering or clearly declare that the target does not support them.
