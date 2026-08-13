# Application and UI delivery plan

This document turns the application/UI vision into tracked implementation
work. It is deliberate about status: **Current** is exercised in the
repository; **WIP** has a bounded implementation slice and acceptance
criteria; **Planned** has not started and must not be advertised as available.

The source-of-truth intent remains [APPLICATION_UI_VISION.md](APPLICATION_UI_VISION.md).

## Invariants

Every delivery item preserves ahead-of-time compilation, no Kalcite VM, no
garbage collector, no hidden allocation, and explicit cost or capacity for
variable-size work. A feature must either lower with bounded memory on a
constrained target, fail clearly for that target, or be isolated in an optional
adapter.

## Build contracts

| Item | Status | Delivery slice | Acceptance criteria |
| --- | --- | --- | --- |
| Profiles: `cli`, `ui`, `game2d`, `embedded`, `wasm` | WIP | `feature/application-contracts` | Manifest round-trips; CLI validates profile names; `ui` requires window and keyboard. |
| Explicit target capabilities | WIP | `feature/application-contracts` | Unknown, unavailable, and profile-required capabilities produce stable diagnostics before source compilation. |
| Minimal linking by profile | Planned | Compiler/backend build planning | A CLI artifact excludes renderer/window crates; build report proves the selected dependency set. |
| Target fallbacks | Planned | Platform adapter contract | A requested fallback is named in the report; no adapter is silently substituted. |

## Scene UI

| Item | Status | Delivery slice | Acceptance criteria |
| --- | --- | --- | --- |
| Static controls, containers, keyboard focus, button signals | Current | Existing scene compiler | Scene validation rejects unknown node properties and invalid static signal targets. |
| Resizable DPI-aware UI surface | Planned | Desktop backend | `ui` profile opens a resizable surface; logical game viewport remains unchanged for `game2d`. |
| Adaptive layout | Planned | Scene layout compiler | Stack, grid, flex, anchors, min/max, padding, alignment, and clipping have deterministic tests. |
| Text pipeline | Planned | Engine assets and renderer | Compiled font asset, wrapping, ellipsis, fallback glyphs, and measured layout work without per-frame heap allocation. |
| Unified input and focus | Planned | Platform API and scene runtime | Pointer, keyboard, gamepad, and touch produce typed UI events; controls remain keyboard usable. |
| Dirty invalidation | Planned | Renderer | Region invalidation produces the same pixels as a forced full redraw; overflow uses full redraw. |
| Accessibility | Planned | Scene metadata and desktop adapter | Role/name/value/state/focus/action metadata is emitted for every supported control. |
| Styles and transitions | Planned | Scene compiler and renderer | Unsupported constrained-target effects fail or use a documented fallback. |
| Bounded virtual lists | Planned | UI node/runtime | Visible cells use a declared pool; a large data set cannot create unbounded node storage. |
| Native UI adapters | Planned | Optional platform adapters | File dialog, clipboard, notification, complex text, and accessibility adapters are linked only when required. |

## State and language

| Item | Status | Delivery slice | Acceptance criteria |
| --- | --- | --- | --- |
| Typed `@bind` functions and invalidation metadata | Planned | Syntax, HIR, scene compiler | Binding calls are type-checked; cycles are diagnostics; generated code has no reflection lookup. |
| Bounded shared models/listeners | Planned | Runtime core | Listener capacity is explicit or statically known; overflow has documented behavior. |
| `Result`, `Option`, `?`, `defer` | Planned | Syntax, HIR, MIR, backend | Parser/typechecker/backend tests cover success, propagation, and deterministic cleanup. |
| Exhaustive `match` and payload enums | Planned | Syntax, typecheck, MIR | Non-exhaustive cases are errors; layout is reported and no heap allocation occurs. |
| Bounded slices/strings and allocation-free iteration | Planned | Standard library and typecheck | Capacity/borrowing rules are tested; generated code has no implicit growth path. |
| Monomorphised generics | Planned | HIR/MIR/backend | Build report identifies instantiated code and its binary-size contribution. |
| Deterministic async/coroutines | Planned | HIR/MIR lowering | Each task lowers to a struct and switch; captured state appears in the memory report. |

## Interoperability and developer experience

| Item | Status | Delivery slice | Acceptance criteria |
| --- | --- | --- | --- |
| C FFI | Planned | Compiler/backend ABI layer | ABI-explicit imports/exports and bounded callbacks compile in a standalone integration test. |
| Rust and assembly escape hatches | Current | Native code path | Target-specific native code remains explicit and compiler-validated. |
| Formatter, diagnostics, LSP | WIP | Existing CLI/LSP crates | Formatting and language-server checks run in CI; new syntax receives source spans and actionable messages. |
| Tests and benchmarks in KLC | WIP | Test runner and CLI | Tests are discoverable and run headlessly; benchmark output is reproducible. |
| Unified build report | Planned | Compiler/CLI/profiler | Report includes artifact/assets, memory/pools/stack, UI counts, capabilities, fallbacks, and debug render metrics. |
| Inspector and UI profiler | Planned | Editor/profiler | Inspector reads compiled scene metadata; profiler reports layout/render/invalidation costs. |

## Platform delivery

| Item | Status | Delivery slice | Acceptance criteria |
| --- | --- | --- | --- |
| NumWorks constrained route | Current | NumWorks backend | `no_std` build and hardware qualification path remain supported. |
| Desktop game runner | Current | Desktop backend | Fixed RGB565 logical framebuffer keeps game output representative of constrained targets. |
| Desktop application backend | Planned | Desktop backend | Separate from game runner; capability matrix only advertises shipped services. |
| CLI runtime | Planned | Standard library/platform adapters | No renderer/window dependencies; command-line arguments and explicit I/O errors are tested. |
| WebAssembly | Planned | Web backend | Native WASM artifact, no Kalcite VM, documented browser capability mapping. |

## Sequencing and gates

1. Complete and merge the application-contract branch before new UI API work.
2. Deliver the resizable desktop surface and report baseline before adaptive
   layout or text features.
3. Deliver layout, text, input, and invalidation as one tested UI foundation.
4. Add bindings, styles, accessibility, virtualization, and native adapters in
   independently reviewable branches.
5. Do not mark a row Current until its acceptance criteria are verified by
   focused tests and relevant target builds.
