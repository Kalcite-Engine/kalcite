# Kalcite

Kalcite is a brace-based compiled language and Rust 2D engine designed first for NumWorks, then for Windows, Linux, macOS, and WebAssembly.

Its principles are: no VM, no GC, no implicit allocation, ergonomic objects lowered to structures and static calls, and predictable memory usage.

## Formats

- `.klc` — Kalcite source;
- `.kco` — **Kalcite Compiled Object**, a versioned intermediate object validated by checksum;
- the final executable depends on the backend: an Epsilon/NumWorks application, desktop binary, or WASM module.

A `.kco` is not an embedded VM: it is a portable build product that currently contains generated `no_std` Rust code. Future versions will use HIR/MIR sections, assets, and relocations.

## Rust workspace

```text
crates/kalcite-syntax              Lexer, parser, and AST
crates/kalcite-object              .kco binary format
crates/kalcite-linter              Reusable lint rules
crates/kalcite-project             Multi-script discovery, manifests, and project diagnostics
crates/kalcite-compiler            Analysis and backend orchestration
crates/kalcite-hir                 Typed HIR and function bodies
crates/kalcite-mir                 Portable MIR and memory budget
crates/kalcite-backend-rust        Generic native Rust generation
crates/kalcite-backend-numworks    EADK / NumWorks adapter
crates/kalcite-backend-desktop     Dependency-free desktop smoke-test backend
crates/kalcite-runtime-core        no_std static pools and generational handles
crates/kalcite-cli                 `kalcite` CLI
crates/kalcite-engine-core         Portable no_std engine
crates/kalcite-engine-assets       Asset formats and codecs
crates/kalcite-platform-api        Platform contracts
crates/kalcite-platform-headless   Test backend
crates/kalcite-platform-numworks   NumWorks ABI and backend
editors/vscode-kalcite             VS Code extension
editors/zed-kalcite                Zed extension
editors/tree-sitter-kalcite        Shared grammar for Zed
examples/pong                      Example game
```

The crates are maintained together in this workspace so the compiler, runtime,
engine, and backends can evolve in one commit. The documentation and showcase
sites are the repository's only submodules; see [`REPOSITORIES.md`](REPOSITORIES.md).

## Related products

- [Kalcite LSP](https://github.com/Kalcite-Engine/kalcite-lsp) is an independent
  Language Server Protocol implementation that consumes a versioned Kalcite core.
- [Kalcite Editor](https://github.com/Kalcite-Engine/kalcite-editor) is the
  independent native graphical editor for Kalcite projects.

## Usage

```bash
cargo test --workspace
cargo run -p kalcite-cli -- init MonJeu --name MonJeu
cargo run -p kalcite-cli -- project-check examples/platformer
cargo run -p kalcite-cli -- project-build examples/platformer --target numworks
cargo run -p kalcite-cli -- check examples/pong/src/main.klc
cargo run -p kalcite-cli -- lint examples/pong/src/main.klc
cargo run -p kalcite-cli -- build examples/pong/src/main.klc --target numworks
cargo run -p kalcite-cli -- emit-mir examples/pong/src/main.klc
cargo run -p kalcite-cli -- run examples/pong/src/main.klc
```

The last command creates `examples/pong/src/main.kco`.

## NumWorks

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p kalcite-platform-numworks --target thumbv7em-none-eabihf --release
```

The beginner-friendly multi-script system is described in [`docs/SCRIPTING.md`](docs/SCRIPTING.md).

See [`docs/LANGUAGE.md`](docs/LANGUAGE.md), [`docs/ENGINE.md`](docs/ENGINE.md), [`docs/OBJECT_FORMAT.md`](docs/OBJECT_FORMAT.md), and [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Building a NumWorks `.nwa` application

The initial native backend compiles the Pong example into a VM-free EADK application:

```bash
cargo run -p kalcite-cli -- build-nwa examples/pong/src/main.klc --name Pong -o examples/pong/Pong.nwa
```

Requirements: Rustup, Node.js, and `npx`. Icon conversion uses `nwlink@0.0.19` (or an installed `nwlink`). The generated Rust project remains available in `.kalcite/nwa/main/` for inspection.

To generate native sources only:

```bash
cargo run -p kalcite-cli -- build-nwa examples/pong/src/main.klc --name Pong --no-build
```

## Compiler architecture (v0.6)

Kalcite now has a real portable lowering path instead of a Pong-specific native template:

```text
Kalcite .klc -> syntax -> HIR -> MIR -> Rust backend -> platform backend
```

NumWorks is the reference constrained platform. `kalcite-backend-numworks` only maps the portable runtime API to EADK and emits an official-style Rust project; it does not implement its own NWA container format.

```bash
cargo run -p kalcite-cli -- emit-rust examples/pong/src/main.klc
cargo run -p kalcite-cli -- build-app examples/pong/src/main.klc --target numworks --name Pong -o examples/pong/Pong.nwa
```

See `docs/COMPILER_PIPELINE.md`, `docs/BACKENDS.md`, `docs/MEMORY_MODEL.md`, and `docs/NODES.md`.


## Bounded memory

The language exposes pools and handles directly:

```klc
@pool(32)
public class Bullet extend Entity {
    public Vec2fx position;
}

private Pool[Bullet; 32] bullets;
private Handle[Bullet] bullet;
```

`Pool[T; N]` becomes a heap-free `StaticPool<T, N>`. Handles are generational and reject stale references. `kalcite emit-mir` also displays an estimated static budget.

The desktop smoke-test backend compiles the same game without a graphics dependency:

```bash
cargo run -p kalcite-cli -- run examples/pong/src/main.klc
```

It writes `kalcite-frame.ppm`, which is useful for quickly validating the compilation pipeline before a NumWorks build.

## Desktop Play mode

Run a `.klc` game directly in a native development window:

```sh
cargo run -p kalcite-cli -- run examples/pong/src/main.klc --scale 3 --fps 60
```

The desktop runner keeps the NumWorks-oriented 320x240 RGB565 logical display
and only scales it for presentation, so desktop testing stays representative of
the calculator build.


## NumWorks native pipeline

Kalcite deliberately delegates NumWorks packaging/install to the Epsilon SDK
pipeline instead of reimplementing `.nwa` internals.

```text
.klc -> HIR -> MIR -> Rust no_std -> EADK -> ARM relocatable ELF/.nwa
```

Check the host first:

```bash
cargo run -p kalcite-cli -- doctor numworks
```

Build Pong:

```bash
cargo run -p kalcite-cli -- \
  build-app examples/pong/src/main.klc \
  --target numworks \
  --name Pong \
  -o examples/pong/Pong.nwa
```

Build and install directly over USB:

```bash
cargo run -p kalcite-cli -- \
  build-app examples/pong/src/main.klc \
  --target numworks \
  --name Pong \
  -o examples/pong/Pong.nwa \
  --install
```

The generated EADK project is kept under `.kalcite/numworks/<script>/` for
inspection and manual builds.

## NumWorks advanced APIs

Low-level Epsilon integration, manual SVC caveats, Home/OnOff handling and the
unofficial persistent-storage adapter are documented in
[`docs/NUMWORKS_ADVANCED.md`](docs/NUMWORKS_ADVANCED.md). Public EADK is always
preferred when available.

## Hardware qualification app

The reference profiler is entirely orchestrated in KLC:

```bash
cargo run -p kalcite-cli -- run examples/hardware_profiler/src/main.klc --name KProfile --scale 3
```

NumWorks build:

```bash
cargo run -p kalcite-cli -- build-app examples/hardware_profiler/src/main.klc --target numworks --name KProfile -o examples/hardware_profiler/KProfile.nwa
```

It benchmarks timing, integer CPU work, RGB565 draw calls, display readback, RNG, static pools/handles, input and frame pacing, then presents a final PASS/FAIL summary. See `docs/HARDWARE_PROFILER.md`.

## Standard libraries and saves

Kalcite supports compile-time library imports such as `use std.msgpack;`, `use std.save;`, `use std.math;`, and KLC-authored `use std.easing;`. See `docs/LIBRARIES.md`.

The NumWorks backend also includes the unofficial Epsilon document-storage adapter used by the hardware profiler. It validates the live filesystem metadata before mutation and exposes the same `Storage` API as desktop. See `docs/NUMWORKS_STORAGE.md`.

## Native escape hatches

Low-level code can opt out of the normal KLC safety/portability layer when necessary:

```klc
unsafe rust[numworks] {
    core::hint::spin_loop();
}

unsafe asm[numworks] {
    "nop",
    options(nomem, nostack)
}
```

Native Rust can be untargeted when it is portable. Native ASM always requires a target. See `docs/NATIVE_CODE.md`.
