# Repository organization

The [Kalcite Engine](https://github.com/Kalcite-Engine) GitHub organization
hosts Kalcite's public projects.

| Repository | Purpose |
| --- | --- |
| `kalcite` | Main Rust workspace: language, runtime, engine, backends, CLI, and integrations. |
| `kalcite-lsp` | Independent Language Server Protocol implementation, versioned against the Kalcite core. |
| `kalcite-editor` | Independent native graphical editor for Kalcite projects. |
| `kalcite-docs` | Documentation site for users and contributors. |
| `kalcite-website` | Project showcase website. |

## Main workspace

The core crates under `crates/` remain in the same repository. They evolve
together and are verified by one CI pipeline, avoiding versioning and publishing
constraints between tightly coupled compiler, runtime, engine, and backend
components. The LSP and graphical editor are separate products with their own
repositories, CI, and versioned dependencies on the core.

## Site submodules

The `kalcite` repository references both site sources as submodules. To obtain
a complete clone:

```bash
git clone --recurse-submodules https://github.com/Kalcite-Engine/kalcite.git
```

For an existing clone:

```bash
git submodule update --init --recursive
```

Site changes are made in their dedicated repository. The `kalcite` repository
then updates the referenced commit to produce an integrated, reproducible state.
