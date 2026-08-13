# Organisation Git

Ce dossier est le super-projet `kalcite`. Chaque dossier sous `crates/` est aussi un dépôt Git autonome avec son propre commit initial.

Le commit racine enregistre les crates comme **gitlinks** (`mode 160000`). Cela permet de publier chaque crate dans un dépôt séparé, puis d’ajouter leurs URL réelles dans `.gitmodules`.

Les extensions d’éditeur, la grammaire Tree-sitter et les exemples complets sont également initialisés comme dépôts autonomes. `kalcite-project` porte spécifiquement le manifeste, la découverte récursive des `.klc` et la résolution globale des scripts.

## Après publication des dépôts

```bash
git submodule add https://github.com/Kalcite-Engine/kalcite-syntax crates/kalcite-syntax
# répéter pour chaque projet, puis :
git submodule update --init --recursive
```

Dans cette archive locale, les historiques imbriqués sont conservés. Aucun remote fictif n’est configuré.

## Portable compiler crates added in v0.5

- `crates/kalcite-hir`: typed-ish high-level IR and function-body parser.
- `crates/kalcite-mir`: flattened, platform-neutral game program.
- `crates/kalcite-backend-rust`: generic native Rust code generator.
- `crates/kalcite-backend-numworks`: EADK/official NumWorks Rust pipeline adapter.

## Runtime and desktop crates added in v0.6

- `crates/kalcite-runtime-core`: `no_std` typed generational handles and fixed-capacity pools.
- `crates/kalcite-backend-desktop`: dependency-free native smoke-test backend using the same generated game code.
- `examples/pool_demo`: bounded-memory language example for `Pool[T; N]` and `Handle[T]`.

## Hardware qualification suite added in v0.9

- `examples/hardware_profiler`: independent KLC-only qualification application.
- `crates/kalcite-backend-rust`: lowers the portable `Hardware` and extended `Draw` APIs.
- `crates/kalcite-backend-desktop`: host implementation for profiler development.
- `crates/kalcite-backend-numworks`: public EADK mapping for on-device profiling.
