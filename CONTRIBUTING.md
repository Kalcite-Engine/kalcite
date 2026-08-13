# Contributing to Kalcite

Kalcite's current major line is **0.14, code name Kally**. Contributions are
welcome, but the project deliberately protects a few non-negotiable properties:
ahead-of-time compilation, no Kalcite VM, no garbage collector, and no hidden
allocation on constrained targets.

The complete contributor handbook is [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).
It covers the repository layout, compiler pipeline, node and backend work,
fixtures, review expectations, and Kally release/versioning policy.

Before opening a change, run:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo run -p kalcite-cli -- test tests/klc
cargo test -p kalcite-cli --test documentation
```

Use a focused branch and an imperative English commit message, for example
`compiler: reject unresolved scene signals`. Never mix formatting-only changes
with a semantic change.
