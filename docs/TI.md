# TI-83+/TI-84+ (experimental)

Kalcite includes an experimental build path for the monochrome Z80 TI-83 Plus
and TI-84 Plus family (not the TI-84 Plus CE).

```sh
kalcite build-app examples/pong/src/main.klc --target ti --no-build
# or:
kalcite build-ti examples/pong/src/main.klc --name PONG --no-build
```

The generated project is in `.kalcite/ti/<entry>/`. Its `Makefile` uses
[Spasm-ng](https://github.com/alberthdev/spasm-ng):

```sh
cd .kalcite/ti/main
make PROGRAM=PONG
```

This produces a transferable `.8xp` program for TI-Connect CE or another link
tool. TI names are uppercase ASCII alphanumerics, up to eight characters.

## Current scope

This target is deliberately a bootstrap. It emits a real Z80 `.8xp` launcher
so the build, packaging and transfer path can be exercised. Kalcite scene
lowering, the 96×64 monochrome renderer and the calculator key map are not yet
implemented. Desktop and NumWorks remain the runnable game targets.
