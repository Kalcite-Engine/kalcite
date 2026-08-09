# NumWorks advanced integration

Kalcite treats NumWorks as the reference constrained platform, but separates
**public EADK API** from firmware-sensitive tricks.

## Sources

The advanced behaviour documented here is based on the Nwagyu reference pages:

- https://nwagyu.org/reference/apps/storage.html#usage
- https://nwagyu.org/reference/apps/syscalls.html
- https://nwagyu.org/reference/apps/onoff-home.html

A mirrored copy of the documentation is also available at
`https://yaya-cout.github.io/Nwagyu/`.

## Rule: EADK first

External apps communicate with Epsilon through kernel services. Public EADK
functions are the compatibility boundary Kalcite uses by default. The Nwagyu
syscall documentation explicitly recommends preferring EADK whenever an EADK
function exists because raw SVC indexes are not guaranteed to remain stable
between Epsilon versions.

The portable Kalcite API therefore maps:

```klc
System.millis();
System.sleep_ms(16);
Input.held(Key.OK);
Draw.rect(0, 0, 10, 10, Color.White);
```

to public EADK calls on NumWorks and equivalent host functionality on desktop.

## Manual SVC calls

Nwagyu documents direct ARM `svc` calls as an escape hatch for kernel functions
not exposed by EADK. For example, suspend is documented as SVC 44 in the cited
reference. These calls are **firmware-sensitive**.

Kalcite keeps such functionality in the generated NumWorks-only module rather
than the portable engine API:

```klc
NumWorks.unsafe_suspend();
```

That currently lowers to `svc 44` on ARM. Code using the `NumWorks` namespace is
not portable and should be tested on every Epsilon version the game supports.

## Home and On/Off

By default, Epsilon handles Home and On/Off at kernel level for external apps.
Nwagyu describes a workaround involving DFU-related kernel behaviour and
reinitialising the keyboard. This is deliberately **not enabled automatically**
in Kalcite because:

1. it relies on non-EADK kernel behaviour;
2. it can change across Epsilon versions;
3. keyboard/interrupt state must be restored correctly;
4. enabling it globally would make ordinary games less robust.

A future `kalcite-numworks-compat` crate can expose this as an explicit opt-in
once its implementation is covered by firmware/model compatibility tests.

## Storage

EADK does not officially expose the calculator filesystem to external apps.
Kalcite therefore ships an explicitly unofficial adapter derived from the
reverse-engineered Epsilon document-store layout used by the community.

The backend discovers the live filesystem through
`SlotInfo -> UserlandHeader -> storage_address_ram`, validates the SlotInfo,
UserlandHeader, filesystem header and footer magic values, then operates on the
bounded record format `[u16 size][name\0][content]`. It refuses access when the
runtime layout is not recognized. See `NUMWORKS_STORAGE.md`.

```text
Kalcite Storage API
       |
       +-- desktop -> .kalcite-saves/ host files
       |
       +-- NumWorks -> validated Epsilon RAM document-store adapter
```

```klc
Storage.supported();
Storage.write_text("SAVE", "DATA");
Storage.exists("SAVE");
Storage.size("SAVE");
Storage.checksum("SAVE");
Storage.remove("SAVE");
```

Rust standard-library helpers additionally use bounded `write_bytes` /
`read_into` calls for MessagePack without heap allocation. Because this bypasses
the public EADK ABI, firmware changes can break it; applications should check
`Storage.supported()` and important documents should be backed up before testing
new firmware/build combinations.

## Build/toolchain compatibility

Kalcite uses `nwlink@0.0.19` or newer for the NumWorks build/install pipeline.
The Nwagyu application guide notes that newer nwlink versions are needed with
recent Node releases and use `eadk-cflags-device` on the C/C++ side.

The Rust backend keeps the official architecture:

```text
.klc -> HIR -> MIR -> Rust no_std -> thumbv7em-none-eabihf -> relocatable ELF/NWA
```

Kalcite never invents a private `.nwa` container format.
