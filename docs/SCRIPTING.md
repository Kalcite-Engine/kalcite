# Kalcite scripts — beginner workflow

Kalcite treats `.klc` scripts as game components, not Rust files to assemble by
hand. In a project containing `kalcite.toml`, every script under `scripts/` is
discovered recursively.

## Creating a project

```bash
kalcite init MonJeu --name MonJeu
cd MonJeu
kalcite project-check
kalcite project-build --target numworks
```

Generated structure:

```text
MonJeu/
├── kalcite.toml
├── assets/
├── scenes/
│   └── main.kscn
└── scripts/
    ├── main.klc
    ├── player.klc
    └── game.klc
```

## One global class per script

The class name is automatically visible from every other script in the project.
No `use` declaration is required in the normal case.

```kalcite
// scripts/player.klc
@component
public class Player extend Node2D {
    @export
    public fx8 speed = 2;
}
```

```kalcite
// scripts/camera_follow.klc
@component
public class CameraFollow extend Node2D {
    @export
    public Player target;
}
```

The compiler resolves `Player` to the correct script, checks duplicates, and
reports an unknown class with the relevant file.

## Inspector-visible fields

`@export` makes a field editable in the future Kalcite editor's inspector:

```kalcite
@export
public fx8 speed = 2;

@export(range = [1, 10], step = 1)
public u8 lives = 3;

@export(file = "*.png")
public SpriteAsset texture;
```

Values remain statically typed and bounded. The editor does not serialize a
dynamic dictionary.

## Node references

`@node("Name")` requests a reference to a scene node:

```kalcite
@component
public class Main extend Node {
    @node("Player")
    private Player player;

    public void Ready() {
        player.enable();
    }
}
```

The reference is resolved during scene building. On NumWorks, it becomes a
compact handle rather than a string lookup every frame.

## Singletons and autoloads

```kalcite
@autoload
public class Game extend Node {
    public u16 score = 0;
}
```

Every script can then write `Game.score`. The build generates one static storage
location.

## Signals

```kalcite
@component
public class Health extend Node {
    public signal died();
    public signal changed(u8 value);

    public void damage(u8 amount) {
        changed.emit(value);
        if (value == 0) { died.emit(); }
    }
}
```

Connections declared in a scene compile to direct calls. Dynamic connections stay
explicit and may be disallowed by the NumWorks profile.

## Readable scenes

`.kscn` scenes use global script names:

Built-in 2D, collision, and GUI nodes use `type="NodeType"`. Their catalogue and
properties are described in `NODES.md`.

```ini
[scene]
root = "Main"

[node "Main"]
script = "Main"

[node "Player" parent="Main"]
script = "Player"
position = [20, 120]
speed = 2.0

[connect]
from = "Player/Health.died"
to = "Main.on_player_died"
```

The aim is Unity/Godot-like editing comfort followed by aggressive compilation:
names removed, references transformed into handles, and static connections.

## Simple rules

- place scripts in `scripts/`;
- use one main class per file;
- name `PlayerController` in `player_controller.klc`;
- use another class's name directly;
- prefer `@export`, `@node`, and `@autoload` to a manual registry;
- run `kalcite project-check` for solution-oriented errors.
