# Kalcite build pipelines

Kalcite is a frontend and game-oriented compiler first. Platforms are independent backends.

```text
.klc -> syntax -> HIR -> MIR
                        |-> backend-rust -> backend-numworks -> EADK / official Rust pipeline -> .nwa
                        |-> backend-rust -> backend-desktop  -> native executable
                        `-> future wasm / other embedded backends
```

The MIR does not contain NumWorks calls. `Input`, `Draw`, `Color`, bounded arrays, pools and handles are translated by the selected platform backend.

## NumWorks profile

The NumWorks backend is the strict reference profile:

- `no_std`
- no GC
- no implicit heap allocation
- fixed pools and generational handles
- static scene memory estimated at build time
- RGB565-oriented drawing
- EADK calls isolated in `platform.rs`
- native ARM target `thumbv7em-none-eabihf`

The resulting project follows the Rust external-app style used by the NumWorks ecosystem. Kalcite does not invent a custom `.nwa` container.

## Desktop profile

The dependency-free desktop smoke-test backend uses the exact same generated game code. It renders to a software framebuffer and writes `kalcite-frame.ppm`. This is deliberately simple: it tests language lowering and engine behavior without requiring SDL, winit, GPU libraries, or internet access.

```sh
kalcite run examples/pong/src/main.klc
```

A later graphical desktop platform can replace only `platform.rs`; the language/MIR stays unchanged.
