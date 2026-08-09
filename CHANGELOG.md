# 0.12.0

- Implement engine roadmap foundations across scenes, assets, rendering, physics, input, audio, saves, checking, tests, profiling, packages and MIR optimization.

## v0.11.0 - Native escape hatches

- Add `unsafe rust { ... }` and target-gated `unsafe rust[target] { ... }` statements.
- Add target-required `unsafe asm[target] { ... }` lowering to `core::arch::asm!`.
- Support NumWorks, desktop, Linux, Windows, macOS and Web/WASM guards.
- Add KLC3001/KLC3002 lints so native code is always visible during review.
- Add editor snippets/highlighting and a native escape example.
- Document the remaining production-engine roadmap.

## v0.10.6 - Tear-resistant NumWorks presentation

- Change the default NumWorks game cadence from ~60 FPS to ~30 FPS.
- Queue KLC drawing for the whole frame and present only after the LCD VBlank.
- Make VBlank authoritative on the ~50 Hz panel instead of racing the scanout.
- Synchronize `Draw.pixel_at()` readback before forcing a pending presentation.
- Keep the bounded dirty-rectangle renderer, clipping hardening, cached input and no-full-framebuffer design from v0.10.5.

## v0.10.5 - NumWorks renderer hardening

- Fix a hardware crash introduced by v0.10.4 when dirty rectangles inherited unclipped text bounds.
- Clip every rectangle again at the final EADK ABI boundary before touching the LCD.
- Store only fully visible text glyph runs in the display list; never queue off-screen text bounds.
- Re-clip every dirty rectangle after merging and reject invalid regions before converting coordinates to `u16`.
- Fall back to a full redraw whenever text changes, because EADK owns the exact font metrics.
- Keep rectangle-only dirty rendering, one keyboard scan per frame, VBlank pacing, and the no-framebuffer memory model.

## v0.10.3

- Fix generated `game.rs` compilation on desktop and NumWorks by moving codegen lint policy to the `mod game` declaration.
- Keep generated game modules warning-quiet without using an invalid inner attribute.
- Retain the v0.10.2 external-app ABI fix so unsupported battery/USB symbols are never linked.
- Add regression tests for both desktop and NumWorks generated module declarations.

## v0.10.2

- Fix NumWorks `nwlink` installation by removing four unsupported battery/USB EADK imports.
- Add `Hardware.telemetry_supported()` and an explicit profiler fallback on NumWorks.
- Silence Rust warnings that originate purely from generated code while keeping Kalcite lint diagnostics active.
- Add ABI/codegen regression tests.

# Changelog

## v0.10.1 - C#-style typed locals

- add explicit local declarations such as `u32 score = 0;` and `Vec2fx position = ...;`;
- keep `var name = expression;` for local type inference;
- add `const u32 Limit = 3;` while retaining legacy local syntax for compatibility;
- support bounded local types such as `[u8; 16]`, `Handle[Bullet]`, and `Pool[Bullet; 8]`;
- keep class fields in the existing explicit `var field: Type` form so memory layouts remain visually obvious;
- update Tree-sitter, VS Code snippets, stdlib KLC, and hardware-profiler sources to exercise the new syntax.

## v0.10.0 - Libraries, MessagePack and unofficial NumWorks storage

- add compile-time `use` declarations and reject unknown bundled libraries;
- add `kalcite-stdlib` as an independent repository with portable Rust/no_std and KLC libraries;
- add allocation-free MessagePack helpers for `u32`, `i32`, `bool`, and `Vec2fx`;
- add save, math, checksum, bits, fixed-point, color, and KLC easing helpers;
- preserve top-level KLC library functions through HIR/MIR and native Rust codegen;
- implement the reverse-engineered Epsilon document-store layout on NumWorks with runtime magic validation;
- add bounded binary Storage reads/writes on desktop and NumWorks;
- extend the hardware profiler with a real MessagePack storage round-trip;
- add `kalcite libs` and a KLC/Rust library demo;
- keep each changed crate/example versioned in its own Git repository and integrate them through meta-repo gitlinks.

## v0.9.1 - Persistent document storage qualification

- add the portable capability-checked `Storage` namespace to KLC codegen;
- implement real desktop document persistence in `.kalcite-saves/`;
- add create/write/checksum/overwrite/delete and latency tests to the KLC hardware profiler;
- add a two-launch persistence marker test to verify data survives an app restart;
- add PASS/FAIL/SKIP reporting so unavailable NumWorks storage is never faked;
- keep the NumWorks storage adapter disabled until the unofficial implementation is audited.

## v0.9.0 - KLC hardware qualification suite

- add a complete hardware/engine profiler written in KLC;
- add `Draw.text`, `Draw.number`, and RGB565 `Draw.pixel_at`;
- add portable `Hardware` probes for battery, USB, backlight, RNG, and target identification;
- map NumWorks probes to the public EADK ABI;
- add CPU, timing, rendering, readback, RNG, Pool/Handle, input, frame pacing and visual tests;
- add a final on-device PASS/FAIL and benchmark summary page;
- keep Home/Back explicitly OS-owned in the interactive key test.

## v0.8.1 - NumWorks advanced integration

- add portable `System.millis()` and `System.sleep_ms()` lowering;
- isolate manual NumWorks SVC helpers from the portable API;
- document Nwagyu storage, syscall, Home and On/Off constraints;
- document storage as a future audited adapter instead of guessing RAM layouts;
- keep NumWorks packaging on the official EADK/nwlink pipeline.

## 0.7.0

- Interactive desktop Play runner with a native window.
- Real-time portable keyboard input mapping.
- 320x240 RGB565 logical framebuffer preserved across desktop and NumWorks.
- Integer nearest-neighbour scaling (default 3x).
- Fixed-rate game loop (default 60 FPS).
- F12 PPM screenshots.
- Headless frame runner retained for CI and deterministic smoke tests.
- Desktop generated binaries no longer exit after 180 frames by default.

## v0.8.0 - NumWorks pipeline cleanup

- Reworked the NumWorks backend around the official Epsilon Rust sample build model.
- Removed the custom sysroot/RUSTFLAGS hacks that could make `core` disappear.
- Generated NumWorks projects now use an isolated stable Rust toolchain with `thumbv7em-none-eabihf`.
- Added a small explicit EADK ABI module for display, keyboard and timing calls.
- Added `kalcite doctor numworks` to validate the ARM `no_std` toolchain independently of game code.
- Added ELF/EADK section validation before a file is exposed as `.nwa`.
- Added `--install` to send a successful build directly through `nwlink install-nwa`.
- Kept Node 18 isolated for `nwlink` icon conversion/install compatibility.

## 0.10.4 - NumWorks incremental renderer

- Added strict 320x240 clipping for rectangles and bounded small-font text rendering.
- Added a fixed-memory display-list renderer with dirty-rectangle merging and automatic full-redraw fallback.
- Added command-overflow streaming so complex frames never silently lose draw calls.
- Cached the NumWorks keyboard scan once per frame instead of issuing one syscall per key query.
- Added adaptive ~60 Hz frame pacing instead of an unconditional post-vblank sleep.
- Kept rendering heap-free: no 153.6 KiB RGB565 framebuffer is allocated.
