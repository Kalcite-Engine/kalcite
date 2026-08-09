# kalcite-backend-numworks

Native NumWorks/Epsilon backend for Kalcite.

This backend does **not** implement the `.nwa` container itself. It generates a
standalone Rust `no_std` EADK project matching the official NumWorks Rust sample
pipeline:

1. Kalcite MIR is emitted as native Rust.
2. Engine primitives are lowered to the public EADK ABI.
3. `cargo` targets `thumbv7em-none-eabihf`.
4. The linker produces a relocatable ARM ELF (`--relocatable`,
   `-no-gc-sections`).
5. The ELF carries `.rodata.eadk_app_name`, `.rodata.eadk_api_level` and
   `.rodata.eadk_app_icon` sections.
6. `nwlink install-nwa` installs that ELF on a calculator.

The generated game uses fixed/static memory only unless a future Kalcite
feature explicitly opts into another strategy. `Pool[T; N]` and `Handle[T]`
come from `kalcite-runtime-core` and require no heap or GC.

## Advanced Epsilon behaviour

Public EADK calls remain the default compatibility boundary. Manual SVCs are
kept in a generated NumWorks-only module and are intentionally named as unsafe
platform-specific operations. Storage and Home/OnOff workarounds are not
implicitly enabled because Nwagyu documents them as non-EADK behaviour. See
`docs/NUMWORKS_ADVANCED.md` in the Kalcite super-project.
