# Hardware profiler

`examples/hardware_profiler/src/main.klc` is the reference end-to-end qualification program for Kalcite. All test sequencing, pass/fail decisions, benchmark loops, interactive pages and the final report are KLC code. The platform layers only provide primitives that games can use too.

## Portable profiling surface

- `System.millis()` / `System.sleep_ms()`
- `Draw.clear`, `Draw.rect`, `Draw.text`, `Draw.number`, `Draw.pixel_at`
- `Input.held`
- `Pool[T; N]` / `Handle[T]`
- `Hardware.is_numworks()`
- `Hardware.battery_level()` and `Hardware.battery_mv()`
- `Hardware.charging()` and `Hardware.usb_plugged()`
- `Hardware.backlight()`
- `Hardware.random()`

The NumWorks implementation maps these directly to public EADK calls. Desktop implements the same API for development; host-only values such as battery are deterministic placeholders and are clearly identified by `Hardware.is_numworks() == false`.

Home and Back are not included in the interactive key matrix because the NumWorks external-app launcher owns those exit paths.
