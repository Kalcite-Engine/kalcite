# Kalcite language reference — Kally 0.14

This document describes the Kally source language as implemented by the current parser, compiler, and standard project workflow. It is deliberately conservative: syntax is only described as **Current** when it is accepted by the checked-in toolchain. Design ideas that have no stable implementation are collected under [Planned language work](#planned-language-work).

Kalcite is compiled ahead of time. A built program does not embed a Kalcite VM or garbage collector. The project aims to make fixed storage, target-specific code, and other non-portable costs visible in source and build diagnostics.

For CLI commands, see [the CLI reference](../kalcite-docs/src/content/docs/reference/cli.md). For explicit pools and handles, see [the memory model](MEMORY_MODEL.md). For native escape hatches, see [native code](NATIVE_CODE.md).

## Source files and comments

Kalcite source files use the `.klc` extension. The parser accepts line comments and non-nested block comments:

```klc
// A line comment.
/* A block comment. */
```

Nested block comments are rejected. Statements and declarations use braces and semicolons where shown below.

## Modules and imports

`module` and `use` accept dotted identifiers and are resolved at compile time:

```klc
module game.player;

use engine.input;
use game.bullet.Bullet;
```

Project scripts under the manifest’s `scripts_dir` are also discovered by the project compiler. Do not rely on a runtime module loader: there is none.

## Declarations

The canonical declaration order is type before name. Visibility can be `public`, `private`, `protected`, or omitted for internal scope.

```klc
public u16 score = 0;
private Vec2fx position = Vec2fx(10, 20);
public const u8 MaxLives = 3;

public struct Pair {
    public i16 x;
    public i16 y;
}
```

Fixed-size and bounded container types use brackets in their type spelling:

```klc
private [u8; 16] scratch;
private Pool[Bullet; 32] bullets;
private Handle[Bullet] last_bullet;
```

The parser also accepts older prototype forms such as `var name: Type` and `fn name(arg: Type) -> Type`. They remain compatibility input, not canonical examples for Kally documentation.

## Classes, inheritance, and attributes

Classes use `extend` as the canonical inheritance keyword. `extends` is accepted as a compatibility alias by the parser.

```klc
@scene
public class Main extend Game {
    u32 score = 0;

    public void Update() {
        score += 1;
    }

    public void Draw() {
        Draw.clear(Color.Black);
        Draw.number(score, 8, 8, Color.White, Color.Black);
    }
}
```

Attributes have the form `@name` or `@name(arguments)`. The syntax layer preserves attribute names and arguments; each compiler or project feature decides which attributes it recognizes. Current project examples use `@scene` for scene roots and `@pool(N)` for bounded entity capacity.

Nested classes are accepted. The current compiler uses static scene and project metadata; do not infer reflection, dynamic class loading, or implicit virtual dispatch from this syntax.

## Functions and control flow

Canonical functions spell the return type before the name, and parameters use `Type name`:

```klc
public i16 Clamp(i16 value, i16 maximum) {
    if (value < 0) {
        return 0;
    }
    if (value > maximum) {
        return maximum;
    }
    return value;
}
```

Function bodies support calls, member access, array expressions, local declarations, assignments (including compound assignments), unary and binary operators, `if`/`else`, `while`, `break`, `continue`, `defer`, and `return`. Conditions may be written with or without parentheses where the parser permits it.

`defer expression;` runs the expression when the current lexical scope is left. Deferred expressions execute in last-in, first-out order and also run before a `return`; they do not allocate a closure or require a garbage collector.

`break;` is valid only inside a `while` body. It leaves the innermost loop after running the deferred expressions belonging to the loop body and any nested lexical block it leaves; defers registered outside that loop remain active until their own scope is left.

`continue;` is also valid only inside a `while` body. It ends the current iteration after running the deferred expressions belonging to the loop body and any nested lexical block it leaves; defers registered outside the loop remain active.

```klc
public void WriteConfig() {
    var file = Fs.open("config");
    defer Fs.close(file);
    WriteEntries(file);
}
```

Kalcite does not document exceptions, coroutines, automatic dynamic dispatch, or hidden allocation as language features.

## Signals and scenes

Classes may declare typed static signals:

```klc
public class Menu extend Control {
    public signal selected(u16 index);
}
```

Scene validation checks supported node signals against their declared handler signatures. Built-in GUI scenes currently include static controls, containers, focus navigation, and a `Button.pressed` signal. This is a static scene feature, not a general reactive binding system.

## Pools and handles

Entity-like objects should state their capacity explicitly:

```klc
@pool(8)
public class Bullet extend Entity {
    Vec2fx position = Vec2fx(20, 120);
    Vec2fx velocity = Vec2fx(2, 0);
}

Pool[Bullet; 8] bullets;
Handle[Bullet] last_bullet;
```

`Pool[T; N]` and `Handle[T]` are the current bounded runtime model. A full pool does not silently grow a heap allocation; handle validity must be checked by program logic. `kalcite lint` reports missing, zero, malformed, and unusually large pool declarations. See [the lint reference](../kalcite-docs/src/content/docs/reference/lint.md).

## Native escape hatches

Kalcite provides explicit native Rust and assembly blocks for small target-specific work:

```klc
unsafe rust[numworks] {
    core::hint::spin_loop();
}

unsafe asm[numworks] {
    "nop",
    options(nomem, nostack)
}
```

Supported native guards are `numworks`, `desktop`, `linux`, `windows`, `macos`, `web`, and `wasm`. Native assembly always requires a guard. Native blocks are intentionally marked `unsafe`: they bypass Kalcite’s portability and safety analysis, and `kalcite lint` emits `KLC3001` or `KLC3002` when it sees them.

## Cost model and target contracts

The current project model separates a project profile (`cli`, `ui`, `game2d`, `embedded`, or `wasm`) from its required target capabilities. `project-check --report` validates the manifest and reports currently measurable static project data, declared pool capacity, and target capabilities.

This does not mean every cost is already known: native artifact size and target stack analysis are explicitly reported as unavailable until their analysis paths exist. A capability that the selected target does not provide is a build-time error rather than an implicit emulation.

## Planned language work

The following concepts have appeared in earlier design material but are **not** stable Kally language syntax or guarantees:

- `match`, enums (including payload enums), `Result`, `Option`, and `?` propagation;
- generic functions and trait/interface-style dynamic dispatch;
- properties with `get`/`set`, state-machine syntax, and `goto` states;
- generic `spawn` expressions, object literals for construction, and exception/panic policies;
- user-facing FFI declarations such as `extern "C"` blocks;
- compile-time recursion proofs, `@max_depth`, `@packed`, `@align`, and most historic attribute proposals;
- automatic bindings, a reactive dependency graph, virtual lists, rich text, and adaptive application layout.

These may be explored in future Kally work, but they must not be used as present-tense API commitments. The [application and UI roadmap](../kalcite-docs/src/content/docs/roadmap/applications-and-ui.md) tracks the UI side separately.

## Checking source

Use the checked-out CLI as the authority for the version you are building:

```sh
kalcite check path/to/main.klc
kalcite lint path/to/main.klc
kalcite emit-mir path/to/main.klc
```

`check` reports parser/compiler diagnostics; `lint` adds cost and portability guidance; `emit-mir` helps inspect the current lowering path. New language documentation should be accompanied by a parser/compiler test or an executable example before it is presented as **Current**.
