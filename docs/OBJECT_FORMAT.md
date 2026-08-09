# Format objet `.kco`

`.kco` signifie **Kalcite Compiled Object**. Il sépare la compilation du langage de l’édition de liens spécifique à une plateforme.

## En-tête version 1

| Offset | Taille | Champ |
|---:|---:|---|
| 0 | 4 | magie `KCO\0` |
| 4 | 2 | version little-endian |
| 6 | 1 | cible |
| 7 | 1 | flags |
| 8 | 4 | taille du payload |
| 12 | 4 | checksum FNV-1a du payload |

Cibles initiales : portable, NumWorks, desktop et web.

## Évolution prévue

La version 1 encapsule le squelette Rust généré afin de valider tout le pipeline. Le format sectionné prévu contiendra : table des symboles, MIR compacte, données constantes, assets, relocations et manifeste de budgets RAM/flash.

Le runtime cible n’interprète pas le `.kco`. Le linker/backend le transforme en code machine avant distribution.
