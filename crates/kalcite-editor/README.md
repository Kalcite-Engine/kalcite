# Kalcite Editor

Native graphical editor for a Kalcite project directory. It reads `.kscn` scenes
through `kalcite-scene` and node metadata from `kalcite-project`.

```sh
cargo run -p kalcite-editor -- examples/game_project
```

The interface includes the scene hierarchy, a typed inspector, resource browser,
node palette, a 320×240 2D viewport, undo/redo history, immediate validation,
and a diagnostics console.
