# Compiler fixture testing

Run the KLC fixture suite with:

```sh
kalcite test tests/klc
```

The runner recursively discovers `.klc` files. A normal fixture must compile.
Prefix the first non-empty line with `// kalcite: expect-error` when the
fixture must fail compilation. Add text after the directive to require a
diagnostic fragment.

```klc
// kalcite: expect-error expected `}`
public class Broken extend Game {
```

Organise fixtures by behavior (`language/`, `scenes/`, `ui/`,
`diagnostics/`, and `codegen/`). A fixture test should demonstrate one user
visible compiler contract; Rust unit tests remain the right place to test an
individual implementation detail.
