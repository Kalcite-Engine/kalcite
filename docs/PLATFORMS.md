# Plateformes

## Contrat commun

Chaque backend implémente `kalcite_platform_api::Platform` : dimensions, horloge monotone, boutons et présentation RGB565. Les futures extensions (audio, stockage, vibrations) seront des traits séparés afin qu'une plateforme minimale n'embarque pas du code inutilisé.

## NumWorks

Cible Rust : `thumbv7em-none-eabihf`. Résolution native : 320×240. Le backend final doit relier les fonctions écran/clavier/temps offertes par l'environnement d'application Epsilon. Les symboles FFI sont confinés à `kalcite-platform-numworks`.

Le matériel officiel documente un STM32F730V8T6, Cortex-M7 216 MHz, 256 Kio SRAM et une flash Quad-SPI 64 Mbit. Le moteur ne suppose jamais que toute cette mémoire est disponible.

## Desktop

Le backend headless est présent. Un backend fenêtré sera ajouté dans un crate indépendant, probablement SDL3, avec : scaling entier, clavier/manette, audio, capture et overlay de budgets.

## Web

Le backend WebAssembly utilisera le même framebuffer logique. La compilation du jeu reste native WASM : aucune VM Kalcite n'est ajoutée.
