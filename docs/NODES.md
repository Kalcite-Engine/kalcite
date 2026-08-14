# Nodes intégrés

Kalcite provides a static node catalogue for the runtime and future editor. A
built-in node is declared directly in a scene:

```ini
[node "Hitbox" type="CollisionShape2D" parent="World/Player"]
shape = capsule
width = 16
height = 32
radius = 8

[node "Title" type="Label" parent="Hud"]
position = [12, 8]
text = "READY"
color = Yellow
background = Black
```

The compiler validates node type, allowed and required properties, and values.
Parent transforms are accumulated without dynamic allocation.

## Cœur

- `Node`, `Game`, `Scene`
- `Timer`
- `Resource` remains a resource type rather than a scene node.

## Nodes 2D

- `Node2D`, `Entity`, `Marker2D`
- `Sprite2D` (`Sprite` remains a compatible alias)
- `AnimatedSprite2D`
- `Camera2D`
- `TileMap`
- `ParallaxLayer2D`

`Sprite2D`, `AnimatedSprite2D`, and `TileMap` use the project's compiled assets.
`Camera2D` configures the camera before rendering the scene.

```ini
[node "Map" type="TileMap" parent="World"]
map = "level.csv"
tileset = "terrain.png"
tile_width = 16
tile_height = 16
```

## Physique 2D

- `CollisionShape2D`
- `StaticBody2D`
- `CharacterBody2D`
- `Area2D`
- `Fluid2D`
- `RayLight2D`
- `LightOccluder2D`

Available `CollisionShape2D` shapes:

- `rectangle` : `width`, `height`
- `circle` : `radius`
- `capsule` : `width`, `height`, `radius`
- `segment` : dimensions/points bornés
- `polygon`: textual `points` property, compiled with the scene

Common properties are `disabled`, `collision_layer`, and `collision_mask`,
depending on the node. `debug_visible = true` draws the shape bounds. The
deterministic `Physics` API provides AABB and circle queries as well as blocking
movement on X and Y. The physics core also resolves circle-to-circle contacts
and keeps fast AABBs from passing through a thin wall.

### Fluide temps réel

`Fluid2D` runs a bounded simulation on each `Update()`: subpixel gravity,
damping, pressure, and impulses between circular particles, followed by
collisions with all four walls. It uses no dynamic allocation and accepts 1 to
64 particles. On NumWorks, keeping the count around 32 leaves a comfortable
rendering budget.

```ini
[node "Fluid" type="Fluid2D" parent="Main"]
position = [8, 34]
width = 304
height = 198
particles = 64
radius = 4
gravity = 2
damping = 99
restitution = 38
interactive = true
obstacle_x = 152
obstacle_y = 118
obstacle_radius = 20
obstacle_color = Orange
color = Cyan
background = Blue
```

With `interactive = true`, directional keys tilt gravity. The `obstacle_x`,
`obstacle_y`, and `obstacle_radius` properties add a static circular obstacle to
the basin. `examples/fluid_demo` shows the real simulation on desktop and
NumWorks.

### Raytracing 2D

`RayLight2D` casts rays each frame from a table of 32 fixed directions. Every
ray advances up to `length` pixels and stops at the first `LightOccluder2D` it
meets. The computation is deterministic and allocation-free; desktop draws
pixel-accurate segments, while NumWorks uses spaced markers to respect the EADK
display queue.

```ini
[node "Wall" type="LightOccluder2D" parent="Main"]
x = 64
y = 72
width = 192
height = 8

[node "Lamp" type="RayLight2D" parent="Main"]
x = 160
y = 120
rays = 16
length = 110
radius = 90
energy = 75
color = Yellow
```

The complete scene is available in `examples/light_demo`.

### Raytracing 3D

`RayTracer3D` casts one camera ray per logical pixel. `RaySphere3D` uses an
analytic ray–sphere intersection; a checkerboard ground plane, Lambert lighting,
and a shadow ray toward the light source are also calculated. This is real 3D
ray tracing. On NumWorks, the engine renders a 32 × 24 block image in passes of
six blocks: the full image appears in about four seconds, then refreshes without
a computation spike or exceeding the embedded rendering list.

```ini
[node "Renderer" type="RayTracer3D" parent="Main"]
width = 320
height = 240
resolution = 80
ambient = 16

[node "RedSphere" type="RaySphere3D" parent="Main"]
center_x = -42
center_y = 16
center_z = 70
radius = 34
color = Red
```

`examples/raytracer_3d_demo` provides a scene with three spheres and a lit floor.

## GUI

Contrôles :

- `Control`
- `Panel`, `ColorRect`
- `Label`, `Button`
- `TextureRect`, `NinePatchRect`
- `ProgressBar`

Layouts :

- `Container`
- `MarginContainer`
- `HBoxContainer`, `VBoxContainer`
- `GridContainer`
- `CenterContainer`

Controls use integer coordinates and dimensions: `x`, `y`, `position`, `width`,
`height`, `visible`, and `layer`. Containers calculate their children's positions
at compilation time. `Panel`, `ColorRect`, `Label`, `Button`, `TextureRect`, and
`ProgressBar` use the same static calls on desktop and NumWorks.

### Button navigation

`Button` nodes automatically form a mouse-free selector. Initial focus uses
`selected = true`, or the first active button. Up/Down/Left/Right select the
nearest geometric neighbour and wrap when there is none. `OK` activates the
static `pressed` signal:

```ini
[node "Play" type="Button" parent="Menu"]
text = "PLAY"
selected = true
selected_color = Yellow

@signal Menu/Play.pressed -> Main.on_play
```

Buttons with `disabled = true` or invisible buttons are removed from navigation.
Focus and neighbours compile statically, with no runtime dynamic list. This
behaviour is identical on desktop and NumWorks.

## Galerie

The `examples/node_gallery` project combines collision shapes, 2D hierarchy, and
several GUI controls:

```bash
cargo run -p kalcite-cli -- project-check examples/node_gallery
cargo run -p kalcite-cli -- build-app examples/node_gallery --target desktop
```
