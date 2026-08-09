# Kalcite Hardware Profiler

A complete engine/hardware qualification application whose test orchestration, thresholds, state machine, UI and final report are written in `src/main.klc`. Platform backends only expose portable primitives.

## Desktop

```bash
cargo run -p kalcite-cli -- run examples/hardware_profiler/src/main.klc --name KProfile --scale 3
```

## NumWorks

```bash
cargo run -p kalcite-cli -- build-app examples/hardware_profiler/src/main.klc --target numworks --name KProfile -o examples/hardware_profiler/KProfile.nwa
```

Tests: system snapshot, timing/sleep, integer CPU loop, RGB565 draw throughput, display readback, random source, static Pool/Handle invalidation, interactive keyboard smoke test, 60-frame pacing, manual text/color qualification, and persistent document storage qualification.

The storage page runs create/write/read-checksum/overwrite/delete tests and records write/read latency. It also leaves a tiny `KALCITE_PERSIST` marker on the first run; reopening the profiler verifies that the document survived a full application restart, then removes it. If a platform has no persistent-storage backend, the page is explicitly marked SKIP rather than reporting a fake PASS.

