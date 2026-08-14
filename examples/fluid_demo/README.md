# Fluid demo

Real-time 2D simulation running on the Kalcite engine:

- 32 circular particles in fixed memory (within the NumWorks budget);
- subpixel gravity and damping;
- circle-to-circle contact resolution with impulses;
- collisions and bounces against all four walls;
- gravity tilt controlled with the directional keys.

```sh
cargo run -p kalcite-cli -- build-app examples/fluid_demo --target desktop
examples/fluid_demo/.kalcite/bin/FluidDemo.desktop
```
