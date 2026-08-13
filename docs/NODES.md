# Nodes intégrés

Kalcite fournit un catalogue statique de nodes destiné au runtime et au futur éditeur. Un node intégré se déclare directement dans une scène :

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

Le compilateur valide le type du node, les propriétés autorisées, les valeurs et les propriétés obligatoires. Les transforms des parents sont additionnés sans allocation dynamique.

## Cœur

- `Node`, `Game`, `Scene`
- `Timer`
- `Resource` reste un type de ressource et non un node de scène.

## Nodes 2D

- `Node2D`, `Entity`, `Marker2D`
- `Sprite2D` (`Sprite` reste un alias compatible)
- `AnimatedSprite2D`
- `Camera2D`
- `TileMap`
- `ParallaxLayer2D`

`Sprite2D`, `AnimatedSprite2D` et `TileMap` utilisent les assets compilés du projet. `Camera2D` configure la caméra avant le rendu de la scène.

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

Formes disponibles pour `CollisionShape2D` :

- `rectangle` : `width`, `height`
- `circle` : `radius`
- `capsule` : `width`, `height`, `radius`
- `segment` : dimensions/points bornés
- `polygon` : propriété textuelle `points`, compilée avec la scène

Les propriétés communes sont `disabled`, `collision_layer` et `collision_mask` selon le node. `debug_visible = true` dessine les bornes de la forme. L'API déterministe `Physics` fournit les requêtes AABB et cercle ainsi que les déplacements bloquants sur X et Y. Le noyau physique résout également les contacts cercle-cercle et empêche les AABB rapides de traverser une paroi fine.

### Fluide temps réel

`Fluid2D` exécute une simulation bornée à chaque `Update()` : gravité subpixel, amortissement, pression et impulsions entre particules circulaires, puis collisions avec les quatre parois. Il n'utilise aucune allocation dynamique et accepte de 1 à 64 particules. Pour NumWorks, rester autour de 32 particules conserve un budget d'affichage confortable.

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

Avec `interactive = true`, les touches directionnelles inclinent la gravité. Les propriétés `obstacle_x`, `obstacle_y` et `obstacle_radius` ajoutent un obstacle circulaire statique dans le bassin. `examples/fluid_demo` montre la simulation réelle sur desktop et NumWorks.

### Raytracing 2D

`RayLight2D` lance des rayons à chaque frame à partir d'une table de 32 directions fixes. Chaque rayon avance jusqu'à `length` pixels et s'arrête au premier `LightOccluder2D` rencontré. Le calcul est déterministe et sans allocation ; le desktop trace les segments au pixel, tandis que NumWorks utilise des marqueurs espacés pour respecter la file d'affichage EADK.

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

La scène complète est disponible dans `examples/light_demo`.

### Raytracing 3D

`RayTracer3D` lance un rayon de caméra par pixel logique. Les `RaySphere3D` utilisent une intersection analytique rayon–sphère ; un plan de sol quadrillé, l'éclairage Lambert et un rayon d'ombre vers la source sont également calculés. Il s'agit d'un vrai rendu 3D par lancer de rayons. Sur NumWorks, le moteur rend une image de 32 × 24 blocs par passes de 6 blocs : l'image complète apparaît en environ 4 secondes, puis se rafraîchit sans pic de calcul ni dépassement de la liste de rendu embarquée.

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

`examples/raytracer_3d_demo` fournit une scène avec trois sphères et un sol éclairé.

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

Les contrôles utilisent des coordonnées et dimensions entières : `x`, `y`, `position`, `width`, `height`, `visible` et `layer`. Les containers calculent la position de leurs enfants à la compilation. `Panel`, `ColorRect`, `Label`, `Button`, `TextureRect` et `ProgressBar` sont rendus sur desktop et NumWorks par les mêmes appels statiques.

### Navigation des boutons

Les `Button` forment automatiquement un sélecteur sans souris. Le focus initial utilise `selected = true`, ou le premier bouton actif. Les touches Haut/Bas/Gauche/Droite choisissent le voisin géométrique le plus proche et bouclent lorsqu'aucun voisin n'existe. `OK` active le signal statique `pressed` :

```ini
[node "Play" type="Button" parent="Menu"]
text = "PLAY"
selected = true
selected_color = Yellow

@signal Menu/Play.pressed -> Main.on_play
```

Les boutons `disabled = true` ou invisibles sont retirés de la navigation. Le focus et les voisins sont compilés statiquement, sans liste dynamique au runtime. Ce comportement est identique sur desktop et NumWorks.

## Galerie

Le projet `examples/node_gallery` combine les formes de collision, la hiérarchie 2D et plusieurs contrôles GUI :

```bash
cargo run -p kalcite-cli -- project-check examples/node_gallery
cargo run -p kalcite-cli -- build-app examples/node_gallery --target desktop
```
