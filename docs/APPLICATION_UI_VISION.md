# Kalcite: native applications and scene-based UI

## Product direction

Kalcite is evolving from a 2D game engine into a native application environment.
One language, compiler, scene system, and asset pipeline should be able to produce:

- small command-line programs;
- desktop applications with scene-based interfaces;
- 2D games;
- WebAssembly modules; and
- constrained-device applications.

NumWorks remains the constrained reference target, not the product ceiling. It
keeps the design honest: ahead-of-time compilation, no Kalcite VM, no garbage
collector, no mandatory reflection, and no hidden allocation. A variable-cost
operation must expose a capacity, a bound, or an explicit failure strategy.

UI is not a second framework next to the engine. It is a specialised scene:
nodes, rendering, assets, input, animation, diagnostics, and build budgets are
shared with games.

## Delivery status

| Area | Status | Evidence / scope |
| --- | --- | --- |
| AOT native path and no-GC constrained runtime | Current | Syntax -> HIR -> MIR -> Rust/platform backends; NumWorks is `no_std`. |
| Static scenes, GUI nodes, containers, focus and button signals | Current | `Control`, basic controls, static containers and keyboard focus are compiled and validated. |
| Desktop runner | Current | Native development window, currently a fixed 320x240 RGB565 logical surface with keyboard input. |
| Product profiles and target capability validation | WIP | Manifest and `project-check` / `project-build` validate declared profiles and capabilities. |
| CLI-focused standard library and application packaging | Planned | No dedicated CLI runtime or packaging flow yet. |
| Adaptive desktop UI, text shaping, pointer/touch, accessibility | Planned | The current GUI is intentionally bounded and simple. |
| Typed bindings, UI styles, virtual lists and native control adapters | Planned | These require scene/compiler/runtime work described below. |
| WebAssembly application backend | Planned | `web` is a declared target, not a shipped backend. |

## Build contracts

Profiles select a product baseline without changing language semantics:

| Profile | Intended use | Status |
| --- | --- | --- |
| `cli` | Commands, build tools, and native utilities | WIP contract; runtime is planned |
| `ui` | Desktop applications and editors | WIP contract; adaptive UI is planned |
| `game2d` | Interactive games and game tools | Current default project shape |
| `embedded` | Calculators and constrained hardware | WIP contract; bounded runtime is current |
| `wasm` | Browser distribution and sandboxing | WIP contract; backend is planned |

Projects can explicitly require capabilities in `kalcite.toml`:

```toml
[project]
name = "Settings"
target = "desktop"
profile = "ui"
capabilities = "window, keyboard"
```

The compiler validates this contract before source compilation. A target never
silently promises a platform service it does not implement. Current capability
claims are deliberately narrow:

- `desktop`: `window`, `keyboard`;
- `numworks` and TI targets: `keyboard`;
- `portable` objects and `web`: no platform capability promise yet.

The capability vocabulary reserves `gpu`, `pointer`, `gamepad`, `filesystem`,
`network`, `threads`, `audio`, `clipboard`, `native_dialogs`, and
`accessibility` for future adapters. Requiring one before its target provides
it is a build error.

```text
.klc + .kscn + kalcite.toml
             |
             v
 profile/capability validation -> typed scene validation -> HIR/MIR
             |
             +--> cli       (planned runtime)
             +--> ui        (desktop app surface, planned)
             +--> game2d    (current engine route)
             +--> embedded  (bounded target route)
             +--> wasm      (planned backend)
```

## Scene UI roadmap

The existing GUI catalogue is the foundation: `Control`, `Panel`, `ColorRect`,
`Label`, `Button`, `TextureRect`, `NinePatchRect`, `ProgressBar`, and basic
containers. Scene properties, node types, signal paths, and button focus are
already statically checked. This is the right model to extend rather than
replace.

### Phase A: application foundation

1. Finish profile/capability reporting, including required services, active
   fallbacks, static memory, pools, and scene counts.
2. Keep the 320x240 game/embedded viewport, but add a separate resizable,
   DPI-aware desktop surface for the `ui` profile.
3. Stabilise `Control`, containers, focus order, static signals, and scene
   diagnostics as public UI foundations.
4. Add a CLI entry-point contract that does not link window or renderer code.

### Phase B: production UI primitives

1. Add adaptive measure/layout: stack, grid, flex, anchors, min/max size,
   padding, alignment, clipping, and integer/fixed-point lowering where needed.
2. Build a real text pipeline: compiled fonts, fallback glyphs, wrapping,
   ellipsis, alignment, selection, and metrics. Font data must remain an
   explicit compact asset.
3. Normalize input into `pressed`, `changed`, `submitted`, and
   `focus_changed` while preserving keyboard/gamepad navigation without a
   pointer device.
4. Add node-region invalidation with a correct full-redraw fallback. Dirty
   rendering is an optimisation, never a correctness risk.
5. Add accessibility metadata to controls: role, name, value, state, focus
   order, and actions.

### Phase C: richer composition

1. Add themes, styles, transforms, opacity, clipping, and transitions. Any
   expensive effect must have an explicit constrained-target fallback.
2. Add bounded virtual lists: visible cells are allocated from an explicit
   pool; rendering a thousand records must not create a thousand nodes.
3. Add optional native adapters for file dialogs, clipboard, notifications,
   complex text editing, and accessibility bridges. Unused adapters are not
   linked.
4. Add an inspector, scene debugger, and UI profiler.

## Typed state, bindings, and signals

Kalcite should not copy WPF's dynamic binding engine. A future binding is a
typed KLC function and a statically known dependency:

```klc
public String[32] ThemeLabel() {
    return dark_mode ? "Dark" : "Light";
}
```

```ini
[node "ThemeLabel" type="Label" parent="Content"]
text = @bind App.ThemeLabel
```

`@bind` is planned syntax, not current syntax. It will lower to a typed call
and static invalidation metadata; binding cycles will be compile errors. The
first implementation may safely recompute per frame, then optimise with an
explicit dependency graph.

Shared models and listener lists must remain bounded. Static function
references are preferred; dynamic subscriptions need a declared capacity.

## Language and developer experience roadmap

The language must remain small and cost-visible. The following are planned
language features, not yet stable user-facing syntax unless the reference
language documentation says otherwise:

- `Result[T, E]` with `?` for explicit error propagation;
- `Option[T]` for absence, including pool lookup;
- `defer` for deterministic cleanup;
- exhaustive `match`, compact enums, and tagged payload unions;
- value structs with inspectable layout;
- bounded slices and capacity-explicit strings;
- allocation-free iteration over arrays and slices;
- monomorphised generics with code-size reporting;
- deterministic `async`/coroutines lowered into structs and switches, with
  captured state included in the memory report.

The toolchain roadmap includes an official formatter, diagnostics with source
spans and fixes, LSP navigation/rename/hover, KLC tests and benchmarks,
reproducible packages, and a unified `build --report`.

The report should show binary and asset contribution, static RAM, pools, stack
estimate, optional arenas, node/binding/signal counts, requested capabilities,
fallbacks, and debug-time layout/render metrics.

## Native interoperability

Kalcite must use existing native ecosystems instead of trying to replace them.
The planned C FFI surface has ABI-explicit types, `@repr(C)` structs, imported
functions, and bounded callbacks. Rust and assembly remain explicit escape
hatches for target-specific work. Platform integrations are optional adapters:
applications that do not use a service do not link it.

## Guardrails

Desktop support must not turn into an opaque runtime. Every new feature must:

1. have a bounded-memory lowering for constrained targets;
2. be clearly unavailable there and rejected at build time; or
3. live in an optional platform adapter.

This preserves Kalcite's core promise: applications, UIs, games, and embedded
programs use the same native toolchain while retaining understandable costs.
