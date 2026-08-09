# Native escape-hatch example

This example demonstrates the deliberately unsafe native escape hatches.

```klc
unsafe rust[numworks] {
    core::hint::spin_loop();
}

unsafe asm[numworks] {
    "nop",
    options(nomem, nostack)
}
```

`unsafe rust { ... }` without a target is emitted on every backend and therefore must compile everywhere.
Assembly always requires an explicit target.
