# Repository organization

The [Kalcite Engine](https://github.com/Kalcite-Engine) GitHub organization
hosts Kalcite's public projects.

| Repository | Purpose |
| --- | --- |
| `kalcite` | Main Rust workspace: language, runtime, engine, backends, CLI, editor, and integrations. |
| `kalcite-docs` | Documentation site for users and contributors. |
| `kalcite-website` | Project showcase website. |

## Main workspace

The crates under `crates/` remain in the same repository. They evolve together
and are verified by one CI pipeline, avoiding versioning and publishing
constraints between tightly coupled components.

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
