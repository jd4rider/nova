---
title: Scene layer
description: Dynamic collisions, stable entity ids and the Nova scene editor.
---

## Dynamic collisions and the editor

`scene_tick` now also resolves dynamic-vs-dynamic pairs: one pass, separation
along the shallower axis, and an equal-mass velocity exchange scaled by the
pair's smaller restitution. Solids are still swept for fast bodies, so a crate
cannot pass through a wall between ticks.

New in the scene layer:

- `scene_set_position(scene, id, x, y)` and `scene_set_velocity` — move an entity
  without respawning it.
- `scene_set_solid(scene, id, solid)` — promote a crate to structural geometry.
- `scene_set_gravity(scene, gravity)` — flip the world's gravity.
- `scene_despawn` / `scene_alive` — delete and query by stable id.

`render.nova` adds `draw_scene_selected`, which strokes the selected entity, and
`examples/scene-gui` is now an editor: click to select, drag to move, arrows to
nudge, backspace to delete, `N` for a new crate, `S` to toggle solid, `G` for
gravity, plus a live inspector showing id, name, tag, transform, velocity, size,
restitution and body type. Because ids are stable, the inspector keeps pointing
at the same entity while the simulation runs.
