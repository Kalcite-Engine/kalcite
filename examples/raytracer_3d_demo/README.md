# True 3D raytracing demo

This scene calculates, for each logical pixel:

- ray–sphere intersections;
- intersection with a checkerboard ground plane;
- attenuated Lambert lighting;
- shadow rays toward the light source;
- block-based rendering on NumWorks.

```sh
cargo run -p kalcite-cli -- build-app examples/raytracer_3d_demo --target desktop
```
