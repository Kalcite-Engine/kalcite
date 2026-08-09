# Spécification du langage Kalcite 0.1

## 1. But

Kalcite est un langage orienté jeu, compilé en code natif. Sa syntaxe mélange l'ergonomie de GDScript/C#, les types de Rust et des blocs à accolades. Le modèle de coût est explicite : tout ce qui alloue, effectue un dispatch dynamique ou conserve une référence doit apparaître dans le code.

## 2. Modèle d'exécution

- compilation ahead-of-time ;
- aucune VM, aucun bytecode embarqué ;
- aucune réflexion ou chargement dynamique ;
- pas de garbage collector ;
- objets de jeu dans des pools à capacité fixe ;
- constantes et assets placés en mémoire en lecture seule ;
- tableaux et chaînes bornés ;
- erreurs de capacité détectables et jamais transformées en allocation cachée.

## 3. Fichiers et modules

Extension : `.klc`.

```kalcite
module game.player;

use engine.input;
use game.bullet.Bullet;
```

Dans un projet avec `kalcite.toml`, toutes les classes placées sous `scripts/` sont indexées globalement. Le cas courant ne demande donc aucun import manuel : un champ `var player: Player;` référence directement la classe `Player` d’un autre fichier. `module` et `use` restent disponibles pour les bibliothèques avancées. Les liens sont résolus à la compilation et ne produisent aucun travail runtime. Voir `SCRIPTING.md`.

## 4. Commentaires

```kalcite
// commentaire de ligne
/* commentaire de bloc */
```

Les commentaires imbriqués ne sont pas autorisés en 0.1.

## 5. Déclarations

```kalcite
const MAX_LIVES: u8 = 3;
var score: u16 = 0;
```

Les champs restent déclarés de façon explicite avec `var name: Type`, afin que le layout mémoire d'une classe soit immédiatement visible.

Dans un corps de fonction, Kalcite utilise une syntaxe locale proche de C# : le type vient avant le nom. `var` est réservé à l'inférence locale.

```kalcite
u32 score = 0;
Vec2fx position = Vec2fx(10, 20);
Handle[Bullet] bullet;
[u8; 16] scratch;
const u32 MaxRetries = 3;

var elapsed = System.millis();
```

L'ancienne écriture locale `var score: u32 = 0;` reste acceptée pour compatibilité, mais le style recommandé est `u32 score = 0;`.

## 6. Types primitifs

```text
bool
u8 i8 u16 i16 u32 i32
usize isize
f32                 // optionnel selon la cible
fx8 fx16             // fixed-point standard
angle8               // tour complet sur 8 bits
Color565              // RGB565 sur 16 bits
```

Les conversions réductrices sont explicites :

```kalcite
u8 small = large as u8;
```

Les opérations arithmétiques ne promeuvent pas silencieusement vers un type plus gros.

## 7. Tableaux et collections

Tableau fixe :

```kalcite
var path: [Vec2i; 32];
const DAMAGE: [u8; 4] = [1, 2, 4, 8];
```

Collections bornées fournies par le moteur :

```kalcite
var visible: SmallList[EntityId, 32];
var lookup: FixedMap[TileId, Collision, 64];
var name: SmallString[24];
```

Le dépassement renvoie une erreur ou un booléen selon l'API. Aucune collection ne grossit automatiquement.

## 8. Structures

Une `struct` est une valeur sans identité, sans allocation et copiée par valeur.

```kalcite
@packed
struct SpriteFlags {
    palette: u4;
    flip_x: bool;
    flip_y: bool;
    visible: bool;
    damaged: bool;
}
```

`@packed` demande au compilateur de vérifier et publier le layout. Un layout impossible est une erreur.

## 9. Classes et objets

Une `class` possède une identité logique. Sur cible embarquée, ses instances résident normalement dans un pool.

```kalcite
@entity
@pool(16)
class Slime extends Enemy {
    var health: u8 = 3;
    var direction: i8 = 1;
    var position: Vec2fx;

    fn update() -> void {
        position.x += direction;
    }
}
```

L'héritage est simple. Le compilateur l'abaisse en composition de préfixe et appels statiques. RTTI et `virtual` implicite n'existent pas.

## 10. Création et destruction

```kalcite
var bullet := spawn Bullet {
    position: player.position,
    velocity: player.facing.vector() * 3,
};

bullet.destroy();
```

`spawn` retourne un `Handle[Bullet]` générationnel. L'échec de capacité doit être traité lorsque la classe n'est pas marquée `@spawn_infallible`.

```kalcite
match (spawn Bullet { position: p }) {
    Ok(id) => active.push(id),
    Err(PoolFull) => dropped_shots += 1,
}
```

## 11. Fonctions

```kalcite
@inline
fn clamp_health(value: i16, maximum: u8) -> u8 {
    return clamp(value, 0, maximum as i16) as u8;
}
```

Les génériques sont monomorphisés :

```kalcite
fn min[T: Ord](a: T, b: T) -> T { ... }
```

La récursion est interdite par défaut sur les profils embarqués. `@max_depth(N)` permet une récursion prouvée bornée.

## 12. Contrôle de flux

```kalcite
if (health == 0) {
    destroy();
} else {
    flash();
}

while (running) { tick(); }
for (enemy in enemies) { enemy.update(); }
```

`match` est exhaustif :

```kalcite
match (state) {
    State.Idle => wait(),
    State.Walk => move(),
    State.Attack => attack(),
}
```

## 13. Enums

```kalcite
enum Direction: u8 {
    Left,
    Right,
}

enum SpawnResult[T] {
    Ok(T),
    Full,
}
```

Les enums avec données utilisent une représentation tag + payload déterminée à la compilation.

## 14. Propriétés

```kalcite
var health: u8 {
    get { return _health; }
    set(value) { _health = min(value, max_health); }
}
```

Une propriété est du sucre pour des fonctions. Aucun metadata runtime n'est généré.

## 15. Interfaces et dispatch

```kalcite
interface Damageable {
    fn damage(amount: u8) -> void;
}
```

Un appel sur type concret est statique. Le dispatch dynamique doit être explicite :

```kalcite
var target: dyn Damageable;
```

Sur NumWorks, `dyn` utilise une paire handle + table de fonctions compacte. Son usage peut être interdit par profil.

## 16. États

```kalcite
class Slime extends Enemy {
    state Idle {
        enter { timer = 30; }
        update {
            timer -= 1;
            if (timer == 0) { goto Walking; }
        }
    }

    state Walking {
        update {
            position.x += direction;
            if (world.has_wall(position)) { goto Idle; }
        }
    }
}
```

Le compilateur génère un enum compact et un `match`. Aucun objet d'état, coroutine ou closure n'est créé.

## 17. Signaux

Connexion statique :

```kalcite
signal died(entity: EntityId);

@connect(enemy.died)
fn on_enemy_died(entity: EntityId) -> void {
    score += 10;
}
```

Connexion dynamique explicite :

```kalcite
enemy.died.connect_dynamic(callback);
```

Le profil NumWorks peut fixer une capacité globale de callbacks.

## 18. Annotations standards

```text
@entity                 type géré par le monde
@pool(N)                capacité fixe
@inline / @noinline     intention d'optimisation
@cold                    chemin rare
@packed                  layout compact vérifié
@align(N)                alignement explicite
@flash                   stockage lecture seule
@asset(...)              pipeline d'asset host
@scene                   racine de scène compilée
@max_instances(N)        borne d'analyse
@max_depth(N)            borne de récursion
@numworks                code spécifique à la cible
@desktop                 code spécifique desktop
@cfg(feature = "...")   compilation conditionnelle
```

## 19. Assets

```kalcite
@asset("player.png", palette = 16, tile = [8, 8])
const PLAYER: SpriteAsset;

@asset("level.tmx", chunk = [16, 16], compression = Rle)
const LEVEL: TileMapAsset;
```

La conversion est faite sur le PC : quantification, déduplication de tiles, flips, compression et validation de budget.

## 20. Mémoire et profils

```kalcite
profile numworks {
    heap = none;
    stack = 24 KiB;
    frame_arena = 8 KiB;
    allow_float = false;
    allow_dyn = false;
}
```

Le compilateur émet un rapport : code, constantes, pools, stack estimée, buffers et marges.

## 21. Erreurs et panics

Les exceptions n'existent pas. Les opérations faillibles retournent des enums. `panic` est réservé aux violations internes et son comportement dépend du profil (`abort`, écran d'erreur, debug trap).

## 22. FFI

```kalcite
extern "C" {
    fn platform_ticks() -> u32;
}
```

Les types FFI doivent avoir un layout stable.

## 23. Compilation

Pipeline prévu :

```text
source -> tokens -> AST -> HIR typée -> MIR sans sucre
       -> analyses de bornes/layout -> IR backend
       -> Rust/C/LLVM selon backend -> binaire natif
```

Le prototype 0.1 parse la structure du langage. Le backend de production visé est d'abord une génération Rust `no_std`, ce qui réutilise rustc/LLVM, LTO et les targets embarquées sans écrire immédiatement un backend ARM.

## 24. Garanties de coût

Le compilateur doit refuser ou signaler : allocation implicite, copie volumineuse involontaire, dispatch dynamique caché, dépassement de pool prouvable, stack non bornée, asset trop gros, type non disponible sur la cible et chemin de rendu dépassant le budget configuré.

## Locals and bounded runtime objects (v0.6)

Function locals use the same `var` / `const` vocabulary as fields, but live on the native stack:

```klc
fn update() -> void {
    var dx: i16 = 2;
    const limit = 320;
    dx += 1;
}
```

The type may be omitted when an initializer is present. No dictionary or runtime symbol table is created.

Dynamic game objects use bounded containers rather than a garbage-collected heap:

```klc
@pool(32)
class Bullet extends Entity {
    var position: Vec2fx;
}

var bullets: Pool[Bullet; 32];
var bullet: Handle[Bullet];
```

`Pool[T; N]` has a compile-time capacity. `Handle[T]` is a typed generational handle. A full pool returns an invalid handle rather than secretly allocating elsewhere.
