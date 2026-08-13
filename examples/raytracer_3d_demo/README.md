# True 3D raytracing demo

Cette scène calcule, pour chaque pixel logique :

- intersections rayon–sphère ;
- intersection avec un plan de sol quadrillé ;
- éclairage Lambert à atténuation ;
- rayons d'ombre vers la source ;
- rendu par blocs sur NumWorks.

```sh
cargo run -p kalcite-cli -- build-app examples/raytracer_3d_demo --target desktop
```
