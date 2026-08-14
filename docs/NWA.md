# NumWorks `.nwa` backend

## Pipeline

```text
main.klc -> Kalcite validation -> no_std Rust project -> relocatable ARM ELF -> .nwa
```

The target is `thumbv7em-none-eabihf`. The linker options are `--relocatable`
and `-no-gc-sections`, following the official NumWorks Rust model. The binary
contains the following EADK sections:

- `.rodata.eadk_app_name`
- `.rodata.eadk_api_level`
- `.rodata.eadk_app_icon`

The initial backend produces the native Pong demonstration. It embeds no VM, GC,
or interpreter. The future MIR backend will gradually replace the specialized
generator without changing the output format.

## Command

```bash
kalcite build-nwa examples/pong/src/main.klc --name Pong -o Pong.nwa
```

`--no-build` keeps only the generated Rust project.

## Installation

Once produced, the file can be sent through the NumWorks Apps page or with:

```bash
npx --yes nwlink@0.0.16 install-nwa Pong.nwa
```


For firmware-sensitive NumWorks features (manual SVCs, storage, Home/OnOff), see [`NUMWORKS_ADVANCED.md`](NUMWORKS_ADVANCED.md).
