# Pool demo

Small compilation test for Kalcite's bounded-memory types:

- `Pool[Bullet; 8]` is a fixed-capacity static pool.
- `Handle[Bullet]` is a typed generational handle (4 bytes).
- `spawn` never allocates; when full it returns an invalid handle.
- stale handles are rejected after `despawn`.

Try:

```sh
kalcite check examples/pool_demo/src/main.klc
kalcite emit-mir examples/pool_demo/src/main.klc
kalcite build-app examples/pool_demo/src/main.klc --target desktop --name PoolDemo
```
