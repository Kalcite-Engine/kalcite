# Kalcite for VS Code

Support for `.klc`: syntax highlighting, brackets, snippets, tasks, and the
Kalcite language server. Install `kalcite-lsp` on `PATH` (the Kallyup
`developer` or `full` profile does this) and open a `.klc` file. Diagnostics,
completion, hover, definitions, and document symbols are then provided by the
server.

The grammar recognizes deterministic `defer` cleanup statements and both
`while` and fixed-array `for item in items` loops. `kl-defer` inserts a
scope-exit cleanup expression, `kl-break` and `kl-continue` insert loop
control flow, `kl-for` inserts a fixed-array iteration, `kl-text-equals`
inserts allocation-free `Text.equals(value, "literal")` comparison, and
`kl-text-starts-with` inserts allocation-free
`Text.starts_with(value, "prefix")` comparison.

Override `kalcite.languageServer.path` when the binary is not on `PATH`; use
`kalcite.languageServer.args` to supply server arguments. The extension keeps
the LSP output in the **Kalcite Language Server** output channel.

The `kalcite` CLI remains available from the integrated terminal for explicit
`lint`, `check`, `build`, `project-check`, and `project-build` commands.

To package the extension: `npx @vscode/vsce package`.
