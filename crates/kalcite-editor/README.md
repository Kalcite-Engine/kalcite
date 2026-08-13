# Kalcite Editor

Éditeur graphique natif pour un dossier de projet Kalcite. Il lit les scènes `.kscn`
avec `kalcite-scene` et les métadonnées de nœuds de `kalcite-project`.

```sh
cargo run -p kalcite-editor -- examples/game_project
```

L’interface comprend la hiérarchie de scène, l’inspecteur typé, un navigateur de
ressources, la palette de nœuds, une viewport 2D 320×240, l’historique annuler/
rétablir, la validation immédiate et la console de diagnostics.
