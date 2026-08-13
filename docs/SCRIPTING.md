# Scripts Kalcite — flux débutant

Kalcite traite les scripts `.klc` comme des composants de jeu, pas comme des fichiers Rust à assembler manuellement. Dans un projet contenant `kalcite.toml`, tous les scripts placés sous `scripts/` sont découverts récursivement.

## Créer un projet

```bash
kalcite init MonJeu --name MonJeu
cd MonJeu
kalcite project-check
kalcite project-build --target numworks
```

Structure créée :

```text
MonJeu/
├── kalcite.toml
├── assets/
├── scenes/
│   └── main.kscn
└── scripts/
    ├── main.klc
    ├── player.klc
    └── game.klc
```

## Une classe globale par script

Le nom de classe est automatiquement visible depuis tous les autres scripts du projet. Aucun `use` n’est nécessaire pour le cas normal.

```kalcite
// scripts/player.klc
@component
public class Player extend Node2D {
    @export
    public fx8 speed = 2;
}
```

```kalcite
// scripts/camera_follow.klc
@component
public class CameraFollow extend Node2D {
    @export
    public Player target;
}
```

Le compilateur relie `Player` au bon script, vérifie les doublons et signale une classe introuvable avec le fichier concerné.

## Champs visibles dans l’inspecteur

`@export` rend un champ éditable dans l’inspecteur du futur éditeur Kalcite :

```kalcite
@export
public fx8 speed = 2;

@export(range = [1, 10], step = 1)
public u8 lives = 3;

@export(file = "*.png")
public SpriteAsset texture;
```

Les valeurs restent statiquement typées et bornées. L’éditeur ne sérialise pas de dictionnaire dynamique.

## Références vers des nœuds

`@node("Nom")` demande une référence vers un nœud de la scène :

```kalcite
@component
public class Main extend Node {
    @node("Player")
    private Player player;

    public void Ready() {
        player.enable();
    }
}
```

La référence est résolue lors du build de scène. Sur NumWorks, elle devient un handle compact, pas une recherche par chaîne à chaque frame.

## Singletons/autoloads

```kalcite
@autoload
public class Game extend Node {
    public u16 score = 0;
}
```

Tous les scripts peuvent ensuite écrire `Game.score`. Le build génère un stockage statique unique.

## Signaux

```kalcite
@component
public class Health extend Node {
    public signal died();
    public signal changed(u8 value);

    public void damage(u8 amount) {
        changed.emit(value);
        if (value == 0) { died.emit(); }
    }
}
```

Les connexions déclarées dans une scène sont compilées en appels directs. Les connexions dynamiques restent explicites et peuvent être interdites par le profil NumWorks.

## Scènes lisibles

Les scènes `.kscn` utilisent les noms globaux des scripts :

Les nodes intégrés 2D, collision et GUI utilisent `type="NodeType"`. Leur catalogue et leurs propriétés sont décrits dans `NODES.md`.

```ini
[scene]
root = "Main"

[node "Main"]
script = "Main"

[node "Player" parent="Main"]
script = "Player"
position = [20, 120]
speed = 2.0

[connect]
from = "Player/Health.died"
to = "Main.on_player_died"
```

L’objectif est le confort Unity/Godot pendant l’édition, puis une compilation agressive : noms supprimés, références transformées en handles et connexions statiques.

## Règles de simplicité

- placer les scripts dans `scripts/` ;
- utiliser une classe principale par fichier ;
- nommer `PlayerController` dans `player_controller.klc` ;
- utiliser directement le nom d’une autre classe ;
- employer `@export`, `@node` et `@autoload` plutôt qu’un registre manuel ;
- lancer `kalcite project-check` pour obtenir des erreurs orientées solution.
