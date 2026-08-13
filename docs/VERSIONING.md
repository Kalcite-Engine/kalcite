# Kally versioning and releases

Kally is Kalcite's **0.14** major development line. The code name identifies a
coherent product/documentation series; the numeric version remains the machine
readable compatibility identifier.

## Version source of truth

The root `Cargo.toml` owns the workspace version:

```toml
[workspace.package]
version = "0.14.0"
```

Workspace crates must inherit this value. Release notes, the website, CLI
version output, examples, and user documentation must use the same version.

## Compatibility policy

- Patch releases fix bugs and documentation without intentionally changing
  accepted KLC syntax, manifest semantics, or generated object compatibility.
- Minor releases may add backwards-compatible syntax, nodes, capabilities, and
  command options. New optional capability names are still documented as
  unavailable until a target implements them.
- Breaking syntax, manifest, `.kco`, or generated-project changes require an
  explicit migration note and a new major-line decision.

Kalcite has no promise of implicit compatibility between a planned design and
an implementation. A feature is supported only at the scope marked **Current**
in the versioned documentation.

## Documentation versions

Documentation follows the release line, like a Godot-style versioned manual:

- `main` documentation describes the next Kally release under development;
- a release branch/tag freezes the matching docs, examples, and website claims;
- published documentation selects a version from the same release/tag set;
- pages may link to `latest`, but reproducible instructions must identify the
  exact Kally version they target.

Do not copy future commands into a frozen version. If an old page needs a
correction, backport only a factual correction and preserve its feature status.

## Release checklist

1. Create `release/kally-0.14.x` from tested `main`.
2. Set the root workspace version and verify all workspace crates inherit it.
3. Update release notes, `README.md`, technical docs, user manual, and website
   version selector/content together.
4. Run formatting, workspace tests, fixture tests, documentation integrity,
   Clippy, and target-specific smoke tests that the release claims support.
5. Build the documented examples and inspect `project-check --report` for any
   constrained target release.
6. Tag the exact reviewed commit as `v0.14.x` and publish artifacts from that
   tag.
7. Move `main` documentation back to the next development version only after
   the release tag is complete.

The release owner records the commands, target toolchain versions, artifacts,
checksums, and known limitations in the release notes.
