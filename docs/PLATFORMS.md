# Platforms

## Shared contract

Every backend implements `kalcite_platform_api::Platform`: dimensions, monotonic
clock, buttons, and RGB565 presentation. Future extensions (audio, storage, and
haptics) will be separate traits so a minimal platform does not carry unused code.

## NumWorks

Rust target: `thumbv7em-none-eabihf`. Native resolution: 320×240. The final
backend links the screen, keyboard, and time functions exposed by the Epsilon
application environment. FFI symbols are contained in `kalcite-platform-numworks`.

Official hardware documentation lists an STM32F730V8T6, a 216 MHz Cortex-M7,
256 KiB SRAM, and 64 Mbit Quad-SPI flash. The engine never assumes that all of
this memory is available.

## Desktop

The headless backend is available. A windowed backend will be added in an
independent crate, likely using SDL3, with integer scaling, keyboard/controller
input, audio, capture, and budget overlays.

## Web

The WebAssembly backend will use the same logical framebuffer. Game compilation
remains native WASM: no Kalcite VM is added.
