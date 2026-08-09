# Memory model

Kalcite exposes costs that would normally be hidden by a managed game scripting language.

## Static values

Fields have concrete sizes at compile time whenever possible. Examples:

```klc
var health: u8 = 3;
var position: Vec2fx;
var tiles: [u8; 256];
```

## Fixed pools

```klc
@pool(32)
class Bullet extends Entity {
    var position: Vec2fx;
    var velocity: Vec2fx;
}

var bullets: Pool[Bullet; 32];
var bullet: Handle[Bullet];
```

`Pool[T; N]` lowers to `StaticPool<T, N>`. It has no heap fallback. `spawn` returns a typed handle; if the pool is full, the handle is invalid.

Handles contain a slot index and generation. Reusing a slot increments its generation, so an old handle cannot silently point at a new object.

## Local variables

Functions support typed or inferred locals:

```klc
var speed: i16 = 2;
const limit = 100;
```

They lower to ordinary native stack locals. There is no runtime dictionary of variables.

## Build-time budget

`kalcite check` and `kalcite emit-mir` estimate static memory. The estimate is intentionally conservative and is meant to catch obviously impossible NumWorks configurations early.
