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
- [ ] Compile every scene in `scenes_dir`, not only the entry scene
- [ ] Validate exported property names and values against script field types
- [ ] Resolve and validate autoload class declarations
- [x] Generate static node ownership, exported-property initialization, ordered
      lifecycle calls, and typed direct signal dispatch entry points
- [x] Embed compiled scene and asset-pack bytes in desktop and NumWorks binaries
- [ ] Construct autoloads and route language-level `signal.emit(...)` calls into
      the generated direct dispatch entry points

## Integration fixture

- [x] Manifest, multiple scripts, parent/child scene nodes, exported property
      metadata, and a type-checked static signal connection
- [x] Input map, save schema, tilemap CSV, asset pack, object files, compiled
      scene, profiler output, desktop/headless run, and NumWorks build paths
- [ ] Exercise logical input actions in game code instead of physical keys
- [ ] Add a PNG spritesheet with transparency and regions
- [ ] Render sprites, tilemap, camera movement, and multiple layers
- [ ] Exercise physics world integration and collision callbacks
- [ ] Exercise save/load and schema-version failure behavior from game code
- [ ] Exercise autoload lifecycle and direct generated signal delivery
- [ ] Exercise audio through the platform abstraction

## Runtime subsystem status

- Assets: deterministic IDs, PNG decoding, RGB565 conversion, RLE, CSV tile
  import, pack emission, and target embedding exist. Runtime pack lookup,
  deduplicated payloads, and spritesheet metadata remain.
- Renderer: ordering, camera offset, sprite/tilemap command types, and tests exist.
  Asset-backed drawing and the optimized NumWorks RLE run path remain.
- Physics: deterministic AABB blocking exists as an isolated crate. It is not yet
  wired into generated project lifecycle code.
- Input: action maps and pressed/held/released state exist. Generated games do not
  yet load the project map into a shared platform-independent action API.
- Save: schema parsing, typed headers, round trips, and migration detection exist.
  Project-generated typed state and platform storage integration remain.
- Profiler: frame CSV output works. Physics, renderer, dirty-region, asset, and
  static-pool counters remain.
- Packages: deterministic lock/add/remove/sync foundations exist. Dependency
  source materialization and compiler/project integration need further coverage.
- LSP: compiler diagnostics work over stdio. Scene, asset, signal, input-action,
  export, and engine-symbol navigation/completion remain.
- Audio: only the lightweight command/backend abstraction exists.

## Next implementation order

1. Generate and test static entry-scene construction and direct signal wiring.
2. Embed the compiled scene and asset pack in both generated targets.
3. Add asset-backed renderer operations with bounded horizontal RLE runs on
   NumWorks.
4. Wire logical input, fixed-step physics, typed saves, and profiler counters into
   one shared lifecycle.
5. Expand `examples/game_project` for each runtime behavior as it lands.
6. Repeat desktop/headless, native NumWorks, and physical-device validation.
