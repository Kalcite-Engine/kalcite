# Memory model

Kalcite exposes costs that would normally be hidden by a managed game scripting language.

## Static values

Fields have concrete sizes at compile time whenever possible. Examples:

```klc
private u8 health = 3;
private Vec2fx position;
private [u8; 256] tiles;
```

## Fixed pools

```klc
@pool(32)
public class Bullet extend Entity {
    public Vec2fx position;
    public Vec2fx velocity;
}

private Pool[Bullet; 32] bullets;
private Handle[Bullet] bullet;
```

`Pool[T; N]` lowers to `StaticPool<T, N>`. It has no heap fallback. `spawn` returns a typed handle; if the pool is full, the handle is invalid.

Handles contain a slot index and generation. Reusing a slot increments its generation, so an old handle cannot silently point at a new object.

## Local variables

Functions support typed or inferred locals:

```klc
i16 speed = 2;
const i16 limit = 100;
```

They lower to ordinary native stack locals. There is no runtime dictionary of variables.

## Build-time budget

`kalcite check` and `kalcite emit-mir` estimate static memory. The estimate is intentionally conservative and is meant to catch obviously impossible NumWorks configurations early.
