# Kalcite compiler pipeline

Kalcite is a frontend plus portable game IR. A platform backend is not allowed to parse `.klc` itself.

```text
.klc
  -> kalcite-syntax (tokens + declaration AST)
  -> kalcite-hir    (function expressions/statements + types + nested class paths)
  -> kalcite-mir    (flattened classes, resolved platform-neutral program)
  -> backend
       -> kalcite-backend-numworks -> EADK Rust project -> official NumWorks Rust/nwlink pipeline -> .nwa
       -> future desktop backend   -> native desktop executable
       -> future web backend       -> wasm32
```

## Zero-VM rule

There is no bytecode interpreter on the target. `Update()` in a `.klc` file becomes a native Rust method and then native machine code. Platform APIs are lowered to small static wrappers.

## Current generic body support

The HIR parser currently supports calls, field/member access, numeric and boolean expressions, arrays, unary operators, binary arithmetic/comparison/logical operators, assignments and compound assignments, `if/else`, `while`, and `return`.

The next compiler stages will add `match`, enums with payloads, scene references, compile-time pool allocation and stronger type checking.
