# Kalcite v0.14 engine integration checklist

This checklist reflects repository behavior validated on 2026-08-09. It is not a
copy of the roadmap: an item is complete only when its generated or runtime path
works.

## Validated baseline

- [x] `cargo fmt --all -- --check`
- [x] `cargo check --workspace`
- [x] `cargo test --workspace`
- [x] Project discovery, checking, and desktop object generation
- [x] Generated desktop project compilation and 120-frame headless execution
- [x] CSV profile creation in headless mode
- [x] KLC test fixture execution
- [x] Scene demo parsing and static path validation
- [x] Multi-script NumWorks project generation
- [x] Native ARM/NumWorks release build and `.nwa` validation
- [ ] Installation and behavior validation on physical NumWorks hardware

## Scene compiler and static wiring

- [x] Parse INI-style and legacy scene syntax
- [x] Validate node parent paths and scene roots
- [x] Preserve node scripts, parents, properties, connections, and autoloads in
      deterministic KSCN v2 output
- [x] Decode KSCN v2 with bounds, truncation, UTF-8, and trailing-data checks
- [x] Validate scene script classes, signal declarations, receiver methods, and
      signal parameter types against project scripts
- [x] Emit the entry scene as `.kalcite/scenes/<name>.ksc2`
- [x] Compile every scene in `scenes_dir`, not only the entry scene
- [x] Validate exported property names and values against script field types
- [x] Resolve and validate autoload class declarations
- [x] Generate static node ownership, exported-property initialization, ordered
      lifecycle calls, and typed direct signal dispatch entry points
- [x] Embed compiled scene and asset-pack bytes in desktop and NumWorks binaries
- [x] Construct autoloads and route language-level `signal.emit(...)` through
      bounded FIFO queues into generated direct dispatch calls

## Integration fixture

- [x] Manifest, multiple scripts, parent/child scene nodes, exported property
      metadata, and a type-checked static signal connection
- [x] Input map, save schema, tilemap CSV, asset pack, object files, compiled
      scene, profiler output, desktop/headless run, and NumWorks build paths
- [x] Exercise logical input actions in game code instead of physical keys
- [x] Add a PNG spritesheet with transparency and regions
- [x] Render sprites, tilemap, camera movement, and multiple layers
- [x] Exercise physics world integration and collision callbacks
- [x] Exercise save/load and schema-version failure behavior from game code
- [x] Exercise autoload lifecycle and direct generated signal delivery
- [x] Exercise audio through the platform abstraction

## Runtime subsystem status

- Assets: deterministic IDs, PNG decoding, explicit alpha, row-bounded RGB565
  RLE, CSV tile import, deduplicated KAP1 payloads, spritesheet metadata, target
  embedding, and bounded runtime lookup are complete.
- Renderer: stable layers, camera offset, sprites, regions, spritesheet frames,
  tilemaps, clipping, desktop framebuffer drawing, and the optimized NumWorks
  horizontal-run path are complete.
- Physics: deterministic fixed-tick AABB blocking, collision queries, and static
  signal callbacks are wired into generated desktop and NumWorks projects.
- Input: project action maps generate compact target bitmasks with shared
  pressed, held, released, and axis APIs on desktop and NumWorks.
- Save: project schemas generate typed KSAV records and field accessors over the
  shared platform storage API, including schema and future-version rejection.
- Profiler: CSV output includes update, render, physics, draw, dirty-pixel/region,
  sprite, tile, collision-query, pool, and static-RAM counters.
- Packages: deterministic lock/add/remove/sync foundations exist. Dependency
  source materialization and compiler/project integration need further coverage.
- LSP: compiler diagnostics work over stdio. Scene, asset, signal, input-action,
  export, and engine-symbol navigation/completion remain.
- Audio: a lightweight tone/stop abstraction and command accounting are linked
  on both targets, with graceful no-output behavior where hardware audio is
  unavailable.

## Next implementation order

1. Complete dependency source materialization and compiler/project integration.
2. Add engine-aware LSP diagnostics, completion, and navigation.
3. Repeat the full completion audit and native validation.
