# Kally — Git packages

Kally is the package manager for Kalcite projects.

Kalcite packages are source dependencies. A project uses a package only after
the CLI has copied its selected source subtree into `.kally/packages/`; that
directory is compiler input and should not be edited.

`kally.toml` is the editable manifest: it records each requested source and
branch/tag. `kally.lock` is generated from it and records the immutable commit
and checksum used by normal builds. Commit both files; do not commit
`.kally/packages/`.

## Add a package from a monorepo

Use a Git URL prefixed with `git:`. An optional `#SUBDIR` selects one package
from a library monorepo:

```sh
kally add ui \
  git:https://github.com/Kalcite-Engine/kalcite-packages.git#packages/ui \
  v0.3.0
```

The last argument is a branch or tag and defaults to `main`. `kally add`
fetches that ref, resolves it to a commit, materializes the requested subtree,
and writes a lock entry such as:

```text
[ui]
source=git:https://github.com/Kalcite-Engine/kalcite-packages.git#packages/ui
reference=v0.3.0
revision=0123456789abcdef...
checksum=...
```

`reference` is intentionally mutable; `revision` is the commit used by normal
builds, so builds remain reproducible even when a branch advances.

## Sync and update

```sh
kally sync              # materialize exactly the locked commits
kally update            # refresh every Git package from its reference
kally update ui         # refresh one package
```

`kally update` is the only command that moves a Git dependency forward. It
updates the locked commit and checksum after fetching the declared branch or
tag. A commit hash may also be used as the reference when an intentionally
fixed dependency is wanted.

## Local development packages

Local packages remain available for working on two repositories together:

```sh
kally add tween path:../kalcite-packages/packages/tween
```

They are copied and checksummed when added. Use Git dependencies for releases
and shared projects.
