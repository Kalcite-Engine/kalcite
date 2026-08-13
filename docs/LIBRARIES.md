# Kalcite libraries

Kalcite libraries are imported with `use` declarations at the top of a `.klc` file:

```klc
use std.msgpack;
use std.save;
use std.math;
use std.easing;
```

Imports are resolved at compile time. There is no dynamic loader on the target.

## Two implementation kinds

A library can be authored in **KLC** or **Rust**.

* KLC libraries are parsed, lowered to HIR/MIR, optimized and compiled together with the game. `std.easing` is the reference implementation.
* Rust libraries are portable native helpers. They must compile on every backend supported by Kalcite. The bundled Rust standard library is `no_std` compatible; desktop provides `std` only through the platform adapter.

The same source file `kalcite-stdlib/src/portable.rs` is copied into both desktop and NumWorks generated projects, preventing a helper from silently existing on only one target.

## Bundled libraries

### `std.msgpack`

Small allocation-free MessagePack helpers designed for saves:

```klc
use std.msgpack;

MsgPack.write_u32("SCORE", 1200);
u32 score = MsgPack.read_u32("SCORE", 0);
MsgPack.write_i32("X", -42);
MsgPack.write_bool("SEEN_INTRO", true);
MsgPack.write_vec2fx("PLAYER_POS", Vec2fx(12, 80));
```

Supported in this first version: `u32`, `i32`, `bool`, and `Vec2fx`. Encoding uses fixed stack buffers, so no heap or allocator is required.

### `std.save`

Convenience wrapper over MessagePack:

```klc
use std.save;
Save.u32("SCORE", 9001);
u32 score = Save.load_u32("SCORE", 0);
```

### `std.math`

Portable integer helpers (`Math.clamp_i16`, `Math.abs_i16`, `Math.min_u32`, `Math.max_u32`).

### `std.checksum`

Small deterministic checksum helpers.

### `std.bits`, `std.fixed`, `std.color`

Small allocation-free helpers for bit flags, Q8.8 fixed-point multiplication/division, and RGB888 → RGB565 conversion.

### `std.easing`

Implemented entirely in KLC and linked into the program at compile time. It currently exports `step_towards(value, target, step)`.

## Design rules for Rust libraries

A bundled Rust helper must not depend on filesystem, windowing, OS threads, heap allocation, or a target-specific ABI directly. Platform operations go through `crate::platform`. This makes the helper source compile unchanged for desktop and NumWorks.
