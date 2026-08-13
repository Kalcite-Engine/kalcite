# Contributing to Kalcite

This handbook is the contributor reference for **Kalcite 0.14, code name
Kally**. Kally evolves Kalcite from a constrained 2D engine into a native
application environment while retaining its original engineering contract:

- ahead-of-time compilation;
- no Kalcite virtual machine or bytecode interpreter in shipped applications;
- no garbage collector;
- no mandatory runtime reflection; and
- explicit, bounded, or fallible variable-cost operations.

The user-facing direction is documented in
[APPLICATION_UI_VISION.md](APPLICATION_UI_VISION.md). The delivery state and
acceptance criteria are in
[APPLICATION_UI_DELIVERY_PLAN.md](APPLICATION_UI_DELIVERY_PLAN.md). Do not
describe a planned feature as current in code, documentation, or the website.

## Repository map

The root repository is a Rust workspace. Its crates are deliberately small and
have one responsibility each.

| Area | Crates | Responsibility |
| --- | --- | --- |
| Source front end | `kalcite-syntax`, `kalcite-hir`, `kalcite-typecheck` | Tokens, AST, typed bodies, diagnostics. |
| Portable compilation | `kalcite-mir`, `kalcite-mir-opt`, `kalcite-compiler` | Platform-neutral program representation, optimization, orchestration. |
| Project input | `kalcite-project`, `kalcite-scene`, `kalcite-assets`, `kalcite-object` | Manifests, scenes, assets, and versioned compiled objects. |
| Native output | `kalcite-backend-rust`, `kalcite-backend-desktop`, `kalcite-backend-numworks`, `kalcite-backend-ti` | Generate portable Rust then adapt it to a target. |
| Runtime | `kalcite-runtime-core`, `kalcite-engine-core`, `kalcite-renderer`, `kalcite-input`, `kalcite-audio`, `kalcite-save`, `kalcite-physics2d` | Bounded runtime primitives and engine services. |
| Platform boundary | `kalcite-platform-api`, `kalcite-platform-headless`, `kalcite-platform-numworks` | Capability contracts and target implementations. |
| Developer tools | `kalcite-cli`, `kalcite-lsp`, `kalcite-linter`, `kalcite-test-runner`, `kalcite-profiler`, `kalcite-package`, `kalcite-editor` | Commands, editor support, tests, reports, and packaging. |

`examples/` contains executable reference projects. `tests/klc/` contains
language fixtures. `docs/` is technical source documentation. The public
documentation and showcase are maintained in their own repositories and must
be updated in the same change when user-visible behavior changes.

## Local setup

Kally requires Rust **1.85** (edition 2024). Install the formatter and Clippy:

```sh
rustup toolchain install 1.85
rustup component add rustfmt clippy --toolchain 1.85
cargo test --workspace
```

Useful checks while working:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo check --workspace
cargo clippy --workspace --all-targets
cargo run -p kalcite-cli -- test tests/klc
cargo test -p kalcite-cli --test documentation
```

The NumWorks target additionally requires the appropriate Rust target and the
Epsilon/nwlink toolchain. Start with `kalcite doctor numworks`; platform setup
details belong in [PLATFORMS.md](PLATFORMS.md), not in generic compiler code.

## Compilation architecture

The source pipeline is strictly one-way:

```text
.klc / .kscn / kalcite.toml
  -> syntax: lexing and declaration AST
  -> HIR: bodies, expressions, statements, types
  -> type checking and project/scene validation
  -> MIR: resolved, portable program and memory metadata
  -> Rust backend: native source without a Kalcite VM
  -> platform backend: desktop, NumWorks, TI, or another target
```

Read [COMPILER_PIPELINE.md](COMPILER_PIPELINE.md) before changing lowering and
[BACKENDS.md](BACKENDS.md) before changing generated output. A platform backend
must not parse KLC itself and must not redefine language semantics. Put shared
semantics in syntax/HIR/MIR; put target ABI, linking, and platform services in
the platform/backend layer.

### Choosing the correct layer

- Grammar or parsing diagnostic: `kalcite-syntax`.
- Expression/statement representation or name resolution: `kalcite-hir`.
- Portable runtime semantics or memory metadata: `kalcite-mir`.
- Cross-target source emission: `kalcite-backend-rust`.
- EADK, windowing, linker, or target ABI behavior: a platform-specific crate.
- Manifest, profile, capability, asset, or scene validation: the corresponding
  project crate before lowering.

Keep the compiler deterministic: the same inputs and declared target must
produce the same portable program and diagnostics.

## Adding a language feature

1. Write a fixture that states the user-visible syntax and expected result.
2. Extend the parser and AST only as far as required.
3. Lower to HIR and MIR with explicit diagnostics for unsupported cases.
4. Update every backend or reject the feature before backend emission.
5. Add unit tests for the smallest layer that owns the behavior.
6. Document the feature as **Current**, **WIP**, or **Planned** accurately.

Do not use a backend-only special case to implement language behavior. That
would make target choice change the language.

## Adding or changing scene nodes

Scenes are compiled data, not dynamically reflected objects. Follow this order:

1. Define the node contract and bounds in `kalcite-scene`.
2. Validate type, required properties, property values, parent relationship,
   signals, and capacities at project-check time.
3. Add portable MIR representation and static-memory accounting where needed.
4. Implement rendering/runtime behavior in the shared engine layer.
5. Make target-specific adaptation explicit; unsupported target behavior must
   be a diagnostic, not a silent omission.
6. Add a valid fixture and at least one invalid-property or invalid-node test.
7. Update [NODES.md](NODES.md), the public node reference, and capability docs.

Every dynamic collection needs a declared maximum. A node that allocates,
requires a platform service, or has unbounded layout cost needs a visible
capacity, a capability requirement, or an explicit failure path.

## Adding a backend or platform capability

Start at `kalcite-platform-api`: define a small contract that is useful to
portable generated code. Then implement it in a target crate and wire it from
the backend. A capability becomes available only when all of these are true:

1. the platform implementation exists;
2. the manifest validator advertises it for that target;
3. the build report lists it as provided; and
4. a target test or reproducible example exercises it.

Never claim a capability only because an eventual host operating system may
offer it. For example, `filesystem`, `pointer`, `accessibility`, and
`native_dialogs` remain unavailable until their adapters and validation are
implemented. See [APPLICATION_UI_VISION.md](APPLICATION_UI_VISION.md).

## Testing

Run the full workspace suite before submitting. Use tests at three levels:

| Test type | Location | Purpose |
| --- | --- | --- |
| Unit tests | owning crate | One data structure or lowering rule. |
| KLC fixtures | `tests/klc/` | User-visible compile success/failure contract. |
| Example/integration tests | `examples/` or CLI tests | End-to-end command, report, generated program, or backend behavior. |

Fixtures are recursive. A regular `.klc` fixture must compile. Put
`// kalcite: expect-error <diagnostic fragment>` on its first non-empty line
when it must fail. See [TESTING.md](TESTING.md).

Add a regression fixture whenever a bug affected source accepted by users.
Prefer concise fixtures: one behavior, descriptive directory, and no unrelated
engine setup. If diagnostics change intentionally, update the expected fragment
and explain the user-facing improvement in the commit.

## Documentation and public claims

Use English for code comments, commit messages, branch names, technical docs,
public docs, and the website. Source documentation uses repository-relative
Markdown links. The `documentation` integration test checks local Markdown
links and that every workspace crate has a manifest.

When a change affects user behavior, update all applicable sources in the same
branch or coordinated branches:

- `docs/` technical reference;
- user documentation repository;
- showcase website;
- examples and CLI help text.

Use these status words consistently:

| Status | Meaning |
| --- | --- |
| Current | Implemented, supported, and covered by a relevant test or example. |
| WIP | An implemented slice exists, but the documented end state is incomplete. State the exact boundary. |
| Planned | Designed but not shipped; do not give users commands that imply it works. |

## Branch and review workflow

Create one focused branch from `main`:

```sh
git switch main
git pull --ff-only
git switch -c compiler/descriptive-change
```

Use lowercase slash-prefixed categories such as `compiler/`, `scene/`,
`backend/`, `docs/`, `site/`, or `release/`. Keep commits reviewable and
imperative, for example `scene: validate button signal targets`.

Before a pull request, rebase or merge current `main` according to the team’s
policy, run the checks above, and describe:

- user-visible behavior;
- target/profile/capability impact;
- memory or binary-size impact when relevant;
- tests and examples run; and
- documentation/status changes.

Do not combine unrelated refactors, generated artifacts, and feature work.
Do not rewrite another contributor’s uncommitted work. Reviewers should be able
to trace each new behavior from fixture or example through compiler layers to a
documented contract.

## Kally releases and versioning

Kally is the code name for the 0.14 major development line. The canonical
workspace version is `workspace.package.version` in the root `Cargo.toml`;
member crates inherit it with `version.workspace = true`. Do not independently
bump an individual crate unless the workspace model changes deliberately.

See [VERSIONING.md](VERSIONING.md) for compatibility rules, release steps, and
the documentation-version policy. The short version: a release branch and tag
must contain code, docs, examples, and public claims for exactly the same
version. Never retroactively change a versioned documentation branch to claim
a later feature.
