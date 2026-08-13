# Spécification du langage Kalcite 0.1

## 1. But

Kalcite est un langage orienté jeu, compilé en code natif. Sa syntaxe
mélange l'ergonomie de GDScript/C#, les types de Rust et des blocs à
accolades. Le modèle de coût est explicite : tout ce qui alloue,
effectue un dispatch dynamique ou conserve une référence doit apparaître
dans le code.

## 2. Modèle d'exécution

-   compilation ahead-of-time ;
-   aucune VM, aucun bytecode embarqué ;
-   aucune réflexion ou chargement dynamique ;
-   pas de garbage collector ;
-   objets de jeu dans des pools à capacité fixe ;
-   constantes et assets placés en mémoire en lecture seule ;
-   tableaux et chaînes bornés ;
-   erreurs de capacité détectables et jamais transformées en allocation
    cachée.

## 3. Fichiers et modules

Extension : `.klc`.

``` kalcite
module game.player;

use engine.input;
use game.bullet.Bullet;
```

Dans un projet avec `kalcite.toml`, toutes les classes placées sous
`scripts/` sont indexées globalement. Le cas courant ne demande donc
aucun import manuel : un champ `private Player player;` référence
directement la classe `Player` d'un autre fichier. `module` et `use`
restent disponibles pour les bibliothèques avancées. Les liens sont
résolus à la compilation et ne produisent aucun travail runtime. Voir
`SCRIPTING.md`.

## 4. Commentaires

``` kalcite
// commentaire de ligne
/* commentaire de bloc */
```

Les commentaires imbriqués ne sont pas autorisés en 0.1.

## 5. Déclarations

``` kalcite
public const u8 MaxLives = 3;
private u16 score = 0;
```

Les champs utilisent une syntaxe proche de C# :
`[visibility] Type name`, avec une visibilité explicite (`public`,
`private` ou `protected`) lorsque cela améliore l'API. Le type reste
visible dans la déclaration afin que le layout mémoire d'une classe soit
immédiatement compréhensible.

Dans un corps de fonction, le type vient également avant le nom. `var`
est réservé à l'inférence locale.

``` kalcite
u32 score = 0;
Vec2fx position = Vec2fx(10, 20);
Handle[Bullet] bullet;
[u8; 16] scratch;
const u32 MaxRetries = 3;

var elapsed = System.millis();
```

L'ancienne écriture `var score: u32 = 0;` peut rester acceptée
temporairement par le parser pour compatibilité avec les premiers
prototypes, mais elle n'est pas la syntaxe canonique et ne doit pas
apparaître dans les nouveaux exemples ou la documentation.

## 5.1. Syntaxe canonique des classes

La syntaxe canonique de Kalcite suit une forme volontairement proche de
C# pour les déclarations, tout en conservant les types et annotations
propres à Kalcite :

``` kalcite
@scene
public class Pong extend Game {
    public const [u16; 2] Screen = [320, 240];

    @pool(1)
    private class Ball extend Entity {
        public Vec2fx Position = Vec2fx(160, 120);
        public Vec2fx Velocity = Vec2fx(2, 1);

        public void Update() {
            Position += Velocity;
        }
    }

    @pool(2)
    private class Paddle extend Entity {
        public i16 Y = 100;

        public void Update() {
            if (Input.held(Key.Up)) {
                Y -= 2;
            }

            if (Input.held(Key.Down)) {
                Y += 2;
            }
        }
    }

    private Ball ball;
    private Paddle player;
    private Paddle cpu;

    public void Update() {
        player.Update();
        ball.Update();
    }

    public void Draw() {
        Draw.clear(Color.Black);
        Draw.rect(ball.Position.x, ball.Position.y, 8, 8, Color.White);
    }
}
```

Les règles normatives sont donc :

-   héritage avec `extend`, et non `extends` ;
-   déclaration de classe : `[visibility] class Name extend Base` ;
-   déclaration de champ : `[visibility] Type name [= value];` ;
-   constante : `[visibility] const Type Name = value;` ;
-   fonction : `[visibility] ReturnType Name(Type argument, ...)` ;
-   tableaux fixes : `[Type; N]` ;
-   attributs avec `@attribute(...)` ;
-   les accolades et les points-virgules sont obligatoires aux endroits
    montrés par la grammaire ;
-   les API moteur peuvent utiliser des namespaces statiques tels que
    `Input`, `Draw`, `System` et `Storage` ;
-   les méthodes de cycle de vie exposées au moteur utilisent les noms
    `Update()` et `Draw()` dans la documentation canonique.

## 6. Types primitifs

``` text
bool
u8 i8 u16 i16 u32 i32
usize isize
f32                 // optionnel selon la cible
fx8 fx16             // fixed-point standard
angle8               // tour complet sur 8 bits
Color565              // RGB565 sur 16 bits
```

Les conversions réductrices sont explicites :

``` kalcite
u8 small = large as u8;
```

Les opérations arithmétiques ne promeuvent pas silencieusement vers un
type plus gros.

## 7. Tableaux et collections

Tableau fixe :

``` kalcite
private [Vec2i; 32] path;
public const [u8; 4] Damage = [1, 2, 4, 8];
```

Collections bornées fournies par le moteur :

``` kalcite
private SmallList[EntityId, 32] visible;
private FixedMap[TileId, Collision, 64] lookup;
private SmallString[24] name;
```

Le dépassement renvoie une erreur ou un booléen selon l'API. Aucune
collection ne grossit automatiquement.

## 8. Structures

Une `struct` est une valeur sans identité, sans allocation et copiée par
valeur.

``` kalcite
@packed
struct SpriteFlags {
    u4 palette;
    bool flip_x;
    bool flip_y;
    bool visible;
    bool damaged;
}
```

`@packed` demande au compilateur de vérifier et publier le layout. Un
layout impossible est une erreur.

## 9. Classes et objets

Une `class` possède une identité logique. Sur cible embarquée, ses
instances résident normalement dans un pool.

``` kalcite
@entity
@pool(16)
public class Slime extend Enemy {
    public u8 Health = 3;
    private i8 direction = 1;
    public Vec2fx Position;

    public void Update() {
        Position.x += direction;
    }
}
```

L'héritage est simple. Le compilateur l'abaisse en composition de
préfixe et appels statiques. RTTI et `virtual` implicite n'existent pas.

## 10. Création et destruction

``` kalcite
var bullet = spawn Bullet {
    Position: player.Position,
    Velocity: player.Facing.Vector() * 3,
};

bullet.destroy();
```

`spawn` retourne un `Handle[Bullet]` générationnel. L'échec de capacité
doit être traité lorsque la classe n'est pas marquée
`@spawn_infallible`.

``` kalcite
match (spawn Bullet { position: p }) {
    Ok(id) => active.push(id),
    Err(PoolFull) => dropped_shots += 1,
}
```

## 11. Fonctions

``` kalcite
@inline
public u8 ClampHealth(i16 value, u8 maximum) {
    return clamp(value, 0, maximum as i16) as u8;
}
```

Les génériques sont monomorphisés :

``` kalcite
public T Min[T: Ord](T a, T b) { ... }
```

La récursion est interdite par défaut sur les profils embarqués.
`@max_depth(N)` permet une récursion prouvée bornée.

## 12. Contrôle de flux

``` kalcite
if (health == 0) {
    destroy();
} else {
    flash();
}

while (running) { tick(); }
for (enemy in enemies) { enemy.Update(); }
```

`match` est exhaustif :

``` kalcite
match (state) {
    State.Idle => wait(),
    State.Walk => move(),
    State.Attack => attack(),
}
```

## 13. Enums

``` kalcite
enum Direction: u8 {
    Left,
    Right,
}

enum SpawnResult[T] {
    Ok(T),
    Full,
}
```

Les enums avec données utilisent une représentation tag + payload
déterminée à la compilation.

## 14. Propriétés

``` kalcite
public u8 Health {
    get { return health; }
    set(value) { health = min(value, maxHealth); }
}
```

Une propriété est du sucre pour des fonctions. Aucun metadata runtime
n'est généré.

## 15. Interfaces et dispatch

``` kalcite
public interface Damageable {
    public void Damage(u8 amount);
}
```

Un appel sur type concret est statique. Le dispatch dynamique doit être
explicite :

``` kalcite
dyn Damageable target;
```

Sur NumWorks, `dyn` utilise une paire handle + table de fonctions
compacte. Son usage peut être interdit par profil.

## 16. États

``` kalcite
class Slime extend Enemy {
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

Le compilateur génère un enum compact et un `match`. Aucun objet d'état,
coroutine ou closure n'est créé.

## 17. Signaux

Connexion statique :

``` kalcite
public signal Died(EntityId entity);

@connect(enemy.Died)
private void OnEnemyDied(EntityId entity) {
    score += 10;
}
```

Connexion dynamique explicite :

``` kalcite
enemy.died.connect_dynamic(callback);
```

Le profil NumWorks peut fixer une capacité globale de callbacks.

## 18. Annotations standards

``` text
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

``` kalcite
@asset("player.png", palette = 16, tile = [8, 8])
public const SpriteAsset Player;

@asset("level.tmx", chunk = [16, 16], compression = Rle)
public const TileMapAsset Level;
```

La conversion est faite sur le PC : quantification, déduplication de
tiles, flips, compression et validation de budget.

## 20. Mémoire et profils

``` kalcite
profile numworks {
    heap = none;
    stack = 24 KiB;
    frame_arena = 8 KiB;
    allow_float = false;
    allow_dyn = false;
}
```

Le compilateur émet un rapport : code, constantes, pools, stack estimée,
buffers et marges.

## 21. Erreurs et panics

Les exceptions n'existent pas. Les opérations faillibles retournent des
enums. `panic` est réservé aux violations internes et son comportement
dépend du profil (`abort`, écran d'erreur, debug trap).

## 22. FFI

``` kalcite
extern "C" {
    u32 PlatformTicks();
}
```

Les types FFI doivent avoir un layout stable.

## 23. Compilation

Pipeline prévu :

``` text
source -> tokens -> AST -> HIR typée -> MIR sans sucre
       -> analyses de bornes/layout -> IR backend
       -> Rust/C/LLVM selon backend -> binaire natif
```

Le prototype 0.1 parse la structure du langage. Le backend de production
visé est d'abord une génération Rust `no_std`, ce qui réutilise
rustc/LLVM, LTO et les targets embarquées sans écrire immédiatement un
backend ARM.

## 24. Garanties de coût

Le compilateur doit refuser ou signaler : allocation implicite, copie
volumineuse involontaire, dispatch dynamique caché, dépassement de pool
prouvable, stack non bornée, asset trop gros, type non disponible sur la
cible et chemin de rendu dépassant le budget configuré.

## Locals and bounded runtime objects (v0.6)

Function locals live on the native stack. Explicitly typed locals use
`Type name`; `var name = value` is reserved for inference, and constants
use `const Type Name = value`:

``` klc
public void Update() {
    i16 dx = 2;
    const i16 Limit = 320;
    dx += 1;
}
```

`var` may omit the type when an initializer is present. Explicit
declarations keep the type before the name. No dictionary or runtime
symbol table is created.

Dynamic game objects use bounded containers rather than a
garbage-collected heap:

``` klc
@pool(32)
private class Bullet extend Entity {
    public Vec2fx Position;
}

private Pool[Bullet; 32] bullets;
private Handle[Bullet] bullet;
```

`Pool[T; N]` has a compile-time capacity. `Handle[T]` is a typed
generational handle. A full pool returns an invalid handle rather than
secretly allocating elsewhere.
