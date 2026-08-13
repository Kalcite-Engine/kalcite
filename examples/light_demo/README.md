# 2D raytracing demo

Raytracing déterministe et temps réel :

- 16 rayons par frame ;
- direction discrète sur 32 vecteurs fixes ;
- arrêt au premier `LightOccluder2D` rencontré ;
- rendu desktop précis et approximation bornée sur NumWorks ;
- zéro allocation au runtime.

```sh
cargo run -p kalcite-cli -- build-app examples/light_demo --target desktop
examples/light_demo/.kalcite/bin/LightDemo.desktop
```
