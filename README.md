# Kalcite

Kalcite est un langage compilé à accolades et un moteur 2D Rust conçus d’abord pour la NumWorks, puis pour Windows, Linux, macOS et WebAssembly.

Principes : aucune VM, aucun GC, aucune allocation implicite, objets ergonomiques abaissés en structures et appels statiques, mémoire prévisible.

## Formats

- `.klc` — source Kalcite ;
- `.kco` — **Kalcite Compiled Object**, objet intermédiaire versionné et validé par checksum ;
- l’exécutable final dépend du backend : application Epsilon/NumWorks, binaire desktop ou module WASM.

Un `.kco` n’est pas une VM embarquée : c’est un produit de build transportable contenant actuellement le code Rust `no_std` généré. Les versions suivantes utiliseront des sections HIR/MIR, assets et relocations.

## Sous-projets indépendants

```text
crates/kalcite-syntax              Lexer, parser et AST
crates/kalcite-object              Format binaire .kco
crates/kalcite-linter              Règles de lint réutilisables
crates/kalcite-project             Découverte multi-script, manifestes et diagnostics de projet
crates/kalcite-compiler            Analyse et orchestration des backends
crates/kalcite-hir                 HIR typée et corps de fonctions
crates/kalcite-mir                 MIR portable + budget mémoire
crates/kalcite-backend-rust        Génération Rust native générique
crates/kalcite-backend-numworks    Adaptation EADK / NumWorks
crates/kalcite-backend-desktop     Smoke-test desktop sans dépendances
crates/kalcite-runtime-core        Pools statiques + handles générationnels no_std
crates/kalcite-cli                 CLI `kalcite`
crates/kalcite-engine-core         Moteur portable no_std
crates/kalcite-engine-assets       Codecs et formats d’assets
crates/kalcite-platform-api        Contrats de plateforme
crates/kalcite-platform-headless   Backend de tests
crates/kalcite-platform-numworks   ABI et backend NumWorks
editors/vscode-kalcite             Extension VS Code
editors/zed-kalcite                Extension Zed
editors/tree-sitter-kalcite        Grammaire partagée pour Zed
examples/pong                      Jeu exemple
```

Chaque crate possède son propre dépôt Git. Le dépôt racine est un **super-projet** qui référence les commits des sous-dépôts avec des gitlinks, à la manière de sous-modules. Voir [`REPOSITORIES.md`](REPOSITORIES.md).

## Utilisation

```bash
cargo test --workspace
cargo run -p kalcite-cli -- init MonJeu --name MonJeu
cargo run -p kalcite-cli -- project-check examples/platformer
cargo run -p kalcite-cli -- project-build examples/platformer --target numworks
cargo run -p kalcite-cli -- check examples/pong/src/main.klc
cargo run -p kalcite-cli -- lint examples/pong/src/main.klc
cargo run -p kalcite-cli -- build examples/pong/src/main.klc --target numworks
cargo run -p kalcite-cli -- emit-mir examples/pong/src/main.klc
cargo run -p kalcite-cli -- run examples/pong/src/main.klc
```

Le dernier appel produit `examples/pong/src/main.kco`.

## NumWorks

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p kalcite-platform-numworks --target thumbv7em-none-eabihf --release
```

Le système multi-script orienté débutant est décrit dans [`docs/SCRIPTING.md`](docs/SCRIPTING.md).

Voir [`docs/LANGUAGE.md`](docs/LANGUAGE.md), [`docs/ENGINE.md`](docs/ENGINE.md), [`docs/OBJECT_FORMAT.md`](docs/OBJECT_FORMAT.md) et [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Produire une application NumWorks `.nwa`

Le backend natif initial compile l’exemple Pong en application EADK sans VM :

```bash
cargo run -p kalcite-cli -- build-nwa examples/pong/src/main.klc --name Pong -o examples/pong/Pong.nwa
```

Prérequis : Rustup, Node.js et `npx`. La conversion de l’icône utilise `nwlink@0.0.19` (ou un `nwlink` déjà installé). Le projet Rust généré reste disponible dans `.kalcite/nwa/main/` pour inspection.

Pour uniquement générer les sources natives :

```bash
cargo run -p kalcite-cli -- build-nwa examples/pong/src/main.klc --name Pong --no-build
```

## Compiler architecture (v0.6)

Kalcite now has a real portable lowering path instead of a Pong-specific native template:

```text
Kalcite .klc -> syntax -> HIR -> MIR -> Rust backend -> platform backend
```

NumWorks is the reference constrained platform. `kalcite-backend-numworks` only maps the portable runtime API to EADK and emits an official-style Rust project; it does not implement its own NWA container format.

```bash
cargo run -p kalcite-cli -- emit-rust examples/pong/src/main.klc
cargo run -p kalcite-cli -- build-app examples/pong/src/main.klc --target numworks --name Pong -o examples/pong/Pong.nwa
```

See `docs/COMPILER_PIPELINE.md`, `docs/BACKENDS.md`, and `docs/MEMORY_MODEL.md`.


## Mémoire bornée

Le langage expose maintenant les pools et handles directement :

```klc
@pool(32)
class Bullet extends Entity {
    var position: Vec2fx;
}

var bullets: Pool[Bullet; 32];
var bullet: Handle[Bullet];
```

`Pool[T; N]` devient un `StaticPool<T, N>` sans heap. Les handles sont générationnels et rejettent les références périmées. `kalcite emit-mir` affiche aussi une estimation du budget statique.

Le backend desktop de smoke-test permet de compiler le même jeu sans dépendance graphique :

```bash
cargo run -p kalcite-cli -- run examples/pong/src/main.klc
```

Il écrit `kalcite-frame.ppm`, utile pour vérifier rapidement le pipeline de compilation avant un build NumWorks.

## Desktop Play mode

Run a `.klc` game directly in a native development window:

```sh
cargo run -p kalcite-cli -- run examples/pong/src/main.klc --scale 3 --fps 60
```

The desktop runner keeps the NumWorks-oriented 320x240 RGB565 logical display
and only scales it for presentation, so desktop testing stays representative of
the calculator build.


## NumWorks native pipeline

Kalcite deliberately delegates NumWorks packaging/install to the Epsilon SDK
pipeline instead of reimplementing `.nwa` internals.

```text
.klc -> HIR -> MIR -> Rust no_std -> EADK -> ARM relocatable ELF/.nwa
```

Check the host first:

```bash
cargo run -p kalcite-cli -- doctor numworks
```

Build Pong:

```bash
cargo run -p kalcite-cli -- \
  build-app examples/pong/src/main.klc \
  --target numworks \
  --name Pong \
  -o examples/pong/Pong.nwa
```

Build and install directly over USB:

```bash
cargo run -p kalcite-cli -- \
  build-app examples/pong/src/main.klc \
  --target numworks \
  --name Pong \
  -o examples/pong/Pong.nwa \
  --install
```

The generated EADK project is kept under `.kalcite/numworks/<script>/` for
inspection and manual builds.

## NumWorks advanced APIs

Low-level Epsilon integration, manual SVC caveats, Home/OnOff handling and the
unofficial persistent-storage adapter are documented in
[`docs/NUMWORKS_ADVANCED.md`](docs/NUMWORKS_ADVANCED.md). Public EADK is always
preferred when available.

## Hardware qualification app

The reference profiler is entirely orchestrated in KLC:

```bash
cargo run -p kalcite-cli -- run examples/hardware_profiler/src/main.klc --name KProfile --scale 3
```

NumWorks build:

```bash
cargo run -p kalcite-cli -- build-app examples/hardware_profiler/src/main.klc --target numworks --name KProfile -o examples/hardware_profiler/KProfile.nwa
```

It benchmarks timing, integer CPU work, RGB565 draw calls, display readback, RNG, static pools/handles, input and frame pacing, then presents a final PASS/FAIL summary. See `docs/HARDWARE_PROFILER.md`.

## Standard libraries and saves

Kalcite supports compile-time library imports such as `use std.msgpack;`, `use std.save;`, `use std.math;`, and KLC-authored `use std.easing;`. See `docs/LIBRARIES.md`.

The NumWorks backend also includes the unofficial Epsilon document-storage adapter used by the hardware profiler. It validates the live filesystem metadata before mutation and exposes the same `Storage` API as desktop. See `docs/NUMWORKS_STORAGE.md`.

## Native escape hatches

Low-level code can opt out of the normal KLC safety/portability layer when necessary:

```klc
unsafe rust[numworks] {
    core::hint::spin_loop();
}

unsafe asm[numworks] {
    "nop",
    options(nomem, nostack)
}
```

Native Rust can be untargeted when it is portable. Native ASM always requires a target. See `docs/NATIVE_CODE.md`.
