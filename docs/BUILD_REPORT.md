# Build reports

`project-check` and `project-build` can print a structured summary of the
validated project before target-native linking:

```sh
kalcite project-check examples/game_project --report
kalcite project-build examples/game_project --target desktop --report
```

The report is deliberately factual. It includes the selected profile and
target, required and provided capabilities, script/class counts, all discovered
scene counts, asset-pack measurements, and declared `@pool(N)` capacities.

`compiled scenes`, `asset pack`, and `known static project data` are exact
artifact byte counts. The last value is only the sum of compiled scene data and
the encoded asset pack; it is not a claim about final executable size or RAM.

The report explicitly labels native artifact size and stack size as unavailable
until Kalcite has target linker integration and stack analysis. This is
intentional: a cost report must never turn an unknown value into a plausible
but misleading estimate.

Pool capacities are source-level instance counts, not byte estimates. The final
layout of a class is target-specific and belongs in the future target memory
analysis pass.

No fallback adapters are selected by the current build path, so the report
prints `active fallbacks: none`. When platform adapters gain documented
fallbacks, they must be named here rather than selected silently.
