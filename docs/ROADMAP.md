# Roadmap

## M0 — Fondation (présent)

- workspace séparé ;
- syntaxe et parser structurel ;
- diagnostics ;
- core no_std ;
- framebuffer RGB565 ;
- pools générationnels ;
- RLE ;
- headless ;
- ABI NumWorks ;
- Pong minimal.

## M1 — Langage exécutable

- expressions complètes et précédence ;
- symbol table et résolution de modules ;
- types primitifs, structs, fonctions ;
- HIR/MIR ;
- génération de Rust no_std ;
- intégration Cargo automatique ;
- rapport de taille par type et pool.

## M2 — Moteur jouable

- sprites indexés et palettes ;
- tilemaps chunkées ;
- collisions AABB et grille ;
- animation ;
- scènes compilées ;
- backend SDL3 desktop ;
- hot rebuild host sans runtime dynamique cible.

## M3 — NumWorks

- intégration officielle `nwlink` ;
- écran et clavier réels ;
- timer monotone ;
- stockage persistant borné ;
- profil linker et mesure flash/RAM ;
- exemple installé et benchmarké sur matériel.

## M4 — Toolchain assets

- PNG -> RGB565/indexé ;
- réduction de palette ;
- déduplication de tiles et flips ;
- TMX/LDtk -> chunks ;
- choix automatique RLE/LZSS/heatshrink ;
- rapport de budget et visualiseur.

## M5 — Distribution classique

- Windows/Linux/macOS ;
- WebAssembly ;
- manette, audio, scaling ;
- packaging ;
- mêmes sources de jeu, profils d'assets différents.

## Tooling ajouté au prototype 0.2

- extension source `.klc` ;
- objet compilé `.kco` versionné ;
- crate de lint indépendante ;
- commande `kalcite lint` ;
- extension VS Code ;
- extension Zed et grammaire Tree-sitter ;
- dépôts Git indépendants et super-projet racine.
