# The Nova engine: scene, editor, NovaScript

The game engine today is Breakout-shaped: one struct, one pure step, one
renderer. This is the path to a general engine with a Unity-style editor, and
the rule that keeps it buildable: every phase ships a working tool.

## Phase 0 — Scene and components (foundation)

- `libraries/nova-engine/src/scene.nova`: `Entity` (id, name, tags), component
  lists (`Transform`, `Body`, `Sprite`, `Box`), and `Scene` as a value that
  `tick(scene)` returns.
- A fixed 60 Hz tick with `ph_` physics underneath. Determinism is the contract:
  integer millipixels, no wall-clock inside the simulation.
- Spawn/despawn by id with a free list, not list compaction, so references stay
  valid for a frame.
- Acceptance: a scene of 200 falling boxes runs 10,000 ticks with no drift and
  no allocation growth.

## Phase 1 — Editor

- `examples/nova-editor` on `nova-gui`: hierarchy tree, inspector bound to the
  selected entity's components, scene view with pan/zoom and a grid.
- Play/pause/step, with the simulation state snapshotted on Play so Stop
  restores it exactly.
- Gizmos drawn from component data (transform handles, collider outlines),
  selection by click in the scene view.
- Acceptance: create an entity, add a body and a box, press Play, watch it fall,
  press Stop, and the scene is unchanged.

## Phase 2 — NovaScript

"NovaScript" is **Nova**, restricted by convention rather than by a second
grammar: one file per behaviour, lifecycle hooks only, no host access except an
engine API:

```nova
import "../../libraries/nova-engine/src/api.nova" as api

fn on_start(entity: Int) {
    api.set_velocity(entity, 0, -4000)
}

fn on_update(entity: Int, tick: Int) {
    // nothing yet
}

fn on_collision(entity: Int, other: Int) {
    if api.tag_of(other) == "brick" {
        api.destroy(other)
        api.add_score(10)
    }
}
```

- Compiles ahead of time with the normal compiler into the game binary. No
  interpreter, no `eval`, typed and debuggable on day one.
- The engine calls hooks by name from a generated dispatch table.
- Acceptance: Breakout reimplemented as scripts, with the engine binary
  unchanged between levels.

## Phase 3 — Visual scripting

- A node graph per entity; each node is a function the compiler already knows.
- The graph **generates NovaScript source**, which is compiled, so there is one
  execution model and one debugger.
- Acceptance: a graph that moves an entity and scores on collision produces the
  same source a human would have written.

## Non-goals for now

- 3D, skeletal animation, prefabs, asset import, multiplayer.
- A second runtime for scripts: if the graph and the text compile to the same
  Nova, both stay fast and debuggable.

## Why this order

Physics is done (see `libraries/game.md`), so the scene layer can be thin. The
editor comes before NovaScript because hooks need entities to attach to, and
visual scripting last because it emits the same code as text scripts.
