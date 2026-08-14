# Kalcite engine architecture

## Goal

A deterministic, portable, `no_std` 2D engine whose minimum capability level is
defined by NumWorks. Desktop backends are development and distribution tools,
not an excuse to introduce dependencies that cannot run on a calculator.

## Workspace components

1. **kalcite-syntax**: no engine dependency.
2. **kalcite-compiler**: host pipeline; `std` is allowed.
3. **kalcite-cli**: build UX, reporting, and conversion.
4. **kalcite-engine-core**: `no_std` and platform-free.
5. **kalcite-engine-assets**: allocation-free streaming decoders.
6. **kalcite-platform-api**: framebuffer, input, time, and storage traits.
7. **kalcite-platform-numworks**: Epsilon/nwlink ABI calls.
8. **kalcite-platform-headless**: tests and CI.

Future independent backends include SDL3, WebAssembly/canvas, Android, iOS, and
bare metal.

## Game loop

The simulation uses a fixed timestep:

```text
poll input -> accumulate time -> N fixed updates -> render -> present
```

On NumWorks, the initial profile targets 30 FPS with a 60 Hz simulation when a
game is light enough. The engine allows 30/30 for heavier games.

## Rendering

- logical RGB565 framebuffer;
- mandatory clipping;
- primitives: pixel, line, rectangle, opaque blit, and color-key blit;
- tilemaps and dirty rectangles in upcoming stages;
- no allocation per frame;
- assets pre-converted to the target format.

## Entities

The core provides `Pool<T, N>` with generational handles. Each gameplay type can
have its own pool, avoiding a costly generic ECS. An optional archetype ECS may
be added in a separate crate.

## Math

- screen coordinates in `i16`;
- time in `u32` ticks with wrap-safe arithmetic;
- Q8.8 and Q16.16 fixed point are planned;
- trigonometry via host-generated LUTs;
- floating point is allowed on desktop but not required by the engine API.

## Initial NumWorks budget

A deliberately conservative game budget, separate from the firmware and backend:

```text
Full framebuffer   : backend-dependent; games should not own it
Game stack         : 16–24 KiB
Frame arena        : 4–8 KiB
Gameplay pools     : 16–48 KiB
Tile/chunk cache   : 8–24 KiB
Margin             : mandatory and measured
```

The reference calculator has a 216 MHz Cortex-M7, 256 KiB SRAM, and 8 MiB of
external flash. The official Rust example application uses the
`thumbv7em-none-eabihf` target; the project architecture follows this target
without assuming that all SRAM or flash is available to a game.

## Portability

Games depend only on `kalcite-engine-core` and `kalcite-platform-api`. Backends
implement the same traits. Resolution differences are handled by a logical
viewport and a scaling policy.

## Desktop Play runner

Desktop Play is intentionally an emulator-like host for the portable engine,
not a separate high-resolution rendering path. The logical resolution remains
320x240 RGB565 so visual results stay close to NumWorks. The native window is
only a presentation layer and uses integer nearest-neighbour scaling.

## NumWorks safe incremental renderer

The NumWorks backend does not allocate a full 320x240 RGB565 framebuffer. It records a bounded display list and uses the LCD contents as the persistent previous frame. Rectangle-only changes may be replayed through clipped dirty regions.

Safety takes priority over incremental rendering:

- every draw primitive is clipped before entering the display list;
- every merged dirty region is clipped again;
- the EADK wrapper performs a final 320x240 clamp before calling firmware;
- changed text forces a full redraw because exact glyph metrics belong to Epsilon;
- display-list or dirty-list overflow falls back to a complete frame;
- no negative or oversized coordinate is converted directly to an EADK `u16`.

This makes dirty rendering an optimization only: any ambiguous case must produce the same result as a full redraw rather than attempting a risky partial update.
