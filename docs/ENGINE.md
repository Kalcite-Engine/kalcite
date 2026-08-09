# Architecture du moteur Kalcite

## Objectif

Un moteur 2D déterministe, portable et `no_std`, dont la NumWorks fixe le niveau minimal. Les backends desktop sont des outils de développement et de distribution, pas une excuse pour introduire des dépendances impossibles sur calculatrice.

## Sous-projets indépendants

1. **kalcite-syntax** : aucune dépendance moteur.
2. **kalcite-compiler** : pipeline host, `std` autorisé.
3. **kalcite-cli** : UX de build, rapports et conversion.
4. **kalcite-engine-core** : `no_std`, aucune plateforme.
5. **kalcite-engine-assets** : décodeurs streaming sans allocation.
6. **kalcite-platform-api** : traits de framebuffer, input, temps, stockage.
7. **kalcite-platform-numworks** : appels ABI Epsilon/nwlink.
8. **kalcite-platform-headless** : tests et CI.

Futurs backends indépendants : SDL3, WebAssembly/canvas, Android, iOS et bare metal.

## Boucle de jeu

La simulation utilise un pas fixe :

```text
poll input -> accumulate time -> N updates fixes -> render -> present
```

Sur NumWorks, le profil initial vise 30 FPS avec une simulation à 60 Hz lorsque le jeu est assez léger. Le moteur autorise 30/30 pour les jeux plus lourds.

## Rendu

- framebuffer logique RGB565 ;
- clipping obligatoire ;
- primitives : pixel, ligne, rectangle, blit opaque, blit color-key ;
- tilemaps et dirty rectangles dans les prochaines étapes ;
- aucune allocation par frame ;
- assets déjà convertis au format cible.

## Entités

Le cœur fournit `Pool<T, N>` avec handles générationnels. Chaque type de gameplay peut avoir son propre pool, ce qui évite un ECS générique coûteux. Un ECS archetype optionnel pourra être ajouté dans un crate distinct.

## Math

- coordonnées écran en `i16` ;
- temps en ticks `u32` avec arithmétique wrap-safe ;
- fixed-point Q8.8 et Q16.16 prévus ;
- trigonométrie via LUT host-générée ;
- flottants permis sur desktop mais non requis par l'API du moteur.

## Budget NumWorks de départ

Budget volontairement prudent pour le jeu, distinct du firmware et du backend :

```text
Framebuffer complet : backend-dépendant, éviter de le posséder côté jeu
Stack jeu          : 16–24 KiB
Arena de frame     : 4–8 KiB
Pools gameplay     : 16–48 KiB
Cache tiles/chunks : 8–24 KiB
Marge               : obligatoire et mesurée
```

La calculatrice de référence possède un Cortex-M7 à 216 MHz, 256 Kio de SRAM et 8 Mio de flash externe. L'application Rust officielle d'exemple utilise la cible `thumbv7em-none-eabihf`; l'architecture du projet suit cette cible sans supposer que toute la SRAM ou la flash est disponible pour un jeu.

## Portabilité

Les jeux dépendent uniquement de `kalcite-engine-core` et `kalcite-platform-api`. Les backends implémentent les mêmes traits. Les différences de résolution sont gérées par un viewport logique et une politique de scaling.

## Desktop Play runner

Desktop Play is intentionally an emulator-like host for the portable engine,
not a separate high-resolution rendering path. The logical resolution remains
320x240 RGB565 so visual results stay close to NumWorks. The native window is
only a presentation layer and uses integer nearest-neighbour scaling.

## NumWorks safe incremental renderer

The NumWorks backend does not allocate a full 320x240 RGB565 framebuffer. It records a bounded display list and uses the LCD contents as the persistent previous frame. Rectangle-only changes may be replayed through clipped dirty regions.

Safety takes priority over incremental rendering:

- every draw primitive is clipped before entering the display list;
- every merged dirty region is clipped again;
- the EADK wrapper performs a final 320x240 clamp before calling firmware;
- changed text forces a full redraw because exact glyph metrics belong to Epsilon;
- display-list or dirty-list overflow falls back to a complete frame;
- no negative or oversized coordinate is converted directly to an EADK `u16`.

This makes dirty rendering an optimization only: any ambiguous case must produce the same result as a full redraw rather than attempting a risky partial update.
