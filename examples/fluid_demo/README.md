# Fluid demo

Simulation 2D en temps réel exécutée par le moteur Kalcite :

- 32 particules circulaires en mémoire fixe (budget NumWorks) ;
- gravité et amortissement subpixel ;
- résolution des contacts cercle-cercle avec impulsions ;
- collisions et rebonds sur les quatre parois ;
- inclinaison de la gravité avec les touches directionnelles.

```sh
cargo run -p kalcite-cli -- build-app examples/fluid_demo --target desktop
examples/fluid_demo/.kalcite/bin/FluidDemo.desktop
```
