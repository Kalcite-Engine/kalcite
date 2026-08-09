# Platform backends

## NumWorks (reference constrained target)

`kalcite-backend-numworks` emits a tiny `no_std` Rust application that uses the EADK ABI. It does not invent or serialize the `.nwa` format. The generated project uses the same relocatable ARM link flags as the official NumWorks Rust sample and lets the NumWorks/nwlink tooling consume the result.

Memory policy:

- no heap in the generated platform runtime;
- values are embedded directly in scene structs;
- fixed-capacity pools are represented in MIR metadata (`@pool(N)`);
- no VM, GC, RTTI, reflection or dynamic script loading;
- display uses direct RGB565 EADK calls;
- keyboard reads use the EADK keyboard state bitset.

Generated directory:

```text
.kalcite/nwa/<entry>/
  Cargo.toml
  rust-toolchain.toml
  .cargo/config.toml
  build.rs
  src/main.rs
  src/platform.rs
  src/game.rs
  src/icon.png
```

## Portable backend boundary

Game code emitted by `kalcite-backend-rust` only references this small logical API:

- `Vec2fx`
- `Input::held(Key)`
- `Draw::clear(Color)`
- `Draw::rect(...)`
- `Color`
- `Key`

A desktop or web platform can implement those names without changing the `.klc` gameplay source.

## Desktop development runner

The desktop backend uses the same logical 320x240 RGB565 framebuffer and the
same generated game code as embedded targets. `kalcite run game.klc` opens a
native host window and maps the host keyboard to the portable Kalcite `Input`
API.

Default mapping:

- Arrow keys -> `Key.Left/Up/Down/Right`
- Enter or Space -> `Key.Ok`
- Escape or Backspace -> `Key.Back`
- H -> `Key.Home`
- F12 -> save the logical framebuffer as a PPM screenshot
- Q -> close the desktop runner

The host renderer only converts RGB565 to XRGB8888 for presentation and applies
nearest-neighbour integer scaling. The game still sees a 320x240 framebuffer.

Generated runners also support a CI-friendly mode:

```sh
kalcite-game-desktop --headless --frames 180 --screenshot kalcite-frame.ppm
```
