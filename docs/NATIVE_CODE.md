# Native Rust and assembly escape hatches

Kalcite normally owns the complete gameplay pipeline so it can optimize memory, validate portability and lower the same source to every backend. Native blocks are an explicit escape hatch for hardware work, experiments and libraries that cannot be expressed in KLC yet.

## Native Rust

Portable native Rust is inserted in every generated Rust backend:

```klc
unsafe rust {
    core::hint::black_box(42u32);
}
```

Target-specific Rust uses a bracket target:

```klc
unsafe rust[numworks] {
    let address = 0x2000_0000usize;
    core::hint::black_box(address);
}
```

Supported target names are `numworks`, `desktop`, `linux`, `windows`, `macos`, `web` and `wasm`.

The compiler wraps native Rust in a Rust `unsafe { ... }` block. Kalcite does not inspect, optimize or make memory-safety guarantees about its contents.

## Native assembly

Assembly is the argument list of Rust's `core::arch::asm!` macro and MUST have an explicit target:

```klc
unsafe asm[numworks] {
    "nop",
    options(nomem, nostack)
}
```

It lowers to roughly:

```rust
#[cfg(all(target_arch = "arm", target_os = "none"))]
unsafe {
    core::arch::asm!(
        "nop",
        options(nomem, nostack)
    );
}
```

This means operands, register constraints and `options(...)` use normal Rust `asm!` syntax.

## Rules

- Prefer KLC or a portable Rust library whenever possible.
- Native blocks bypass KLC's memory analysis and most optimizer passes.
- Native ASM always requires a target.
- Untargeted native Rust must compile on every selected build pipeline.
- A target guard removes the native block at Rust compile time on other targets.
- The linter emits KLC3001/KLC3002 whenever native code is present.
- Native code is intentionally spelled `unsafe`: it is not expected to be beginner-facing gameplay code.

For reusable native code, prefer a bundled/project Rust library. Native blocks are for small low-level pieces close to the call site.
