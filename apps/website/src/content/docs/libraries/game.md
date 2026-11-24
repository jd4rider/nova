---
title: Nova Game Engine
description: A small 2D game engine written in Nova, with Breakout as a desktop terminal game built on the TUI runtime.
---

**Status: experimental.** The engine lives in `libraries/nova-game/src/engine.nova` and is plain Nova with no rendering, input or platform code. `examples/breakout` is a desktop terminal game that uses it.

## Run Breakout

```sh
cd examples/breakout
../../target/debug/nova build
./target/nova/breakout
```

`a`/`d` move the paddle, `space` launches the ball, `r` starts over, `q` or `Esc` quits. It runs in the alternate screen with a 16 ms tick.

## Install on macOS

```sh
cd examples/breakout
./install-macos.sh
```

This builds the game, renders the icon from `icon.html` with headless Chrome, assembles `dist/Breakout.app` and copies it to `/Applications`. Double-clicking the icon opens Terminal running the game. The app is unsigned, so the first launch may need right-click → Open.

## The engine

The simulation is a pure function. No globals, no callbacks, no allocation beyond the brick list:

```nova
struct Game {
    ball_x: Int,
    ball_y: Int,
    vx: Int,
    vy: Int,
    paddle: Int,
    lives: Int,
    score: Int,
    status: Int,
    bricks: List<Int>,
}

fn new_game() -> Game
fn launch(game: Game) -> Game
fn step(game: Game, paddle_target: Int) -> Game
fn bricks_left(game: Game) -> Int
```

`step` moves the ball, bounces it off the walls and paddle, clears the brick it hits, updates score and lives, and returns the next `Game`. Positions are integer pixels in an 800×520 field; the brick grid is 10×5 (50 bricks, 10 points each, 500 to clear). Status is `0` ready, `1` playing, `2` won, `3` lost.

Because it is pure, the same engine can be driven by the terminal demo, a test, or a future canvas/WebAssembly host without changes.

## Physics

`libraries/nova-game/src/physics.nova` is the collision core the engine steps
through. It is a swept-AABB solver in integer millipixels (thousandths of a
pixel), so a fast body cannot tunnel through a thin wall and a replay is
identical on every machine.

```nova
struct Body {
    x: Int, y: Int,        // centre, millipixels
    vx: Int, vy: Int,      // per tick, millipixels
    hw: Int, hh: Int,      // half extents
    bounce: Int,           // percent of speed kept on a bounce
    solid: Bool,           // static collider
    alive: Bool,
}

fn ph_body(x, y, w, h) -> Body          // centre in pixels, for a sprite
fn ph_box(left, top, w, h) -> Body      // top-left in pixels, for a level
fn ph_solid_box(left, top, w, h) -> Body
fn ph_sweep(mover, wall) -> Int         // time of impact 0..1000 or ph_miss()
fn ph_resolve(mover, wall) -> Body      // move to contact, reflect, separate
fn ph_clamp(body, left, top, right, bottom) -> Body
fn ph_gravity(body, pull) -> Body
fn ph_speed_clamp(body, limit) -> Body
fn ph_bounce_paddle(mover, paddle, max_vx) -> Body
```

Units matter: velocities are millipixels per tick, so 4000 is four pixels.
`ph_sweep` returns the earliest time of impact in this step, which is what stops
tunnelling; `ph_resolve` separates the bodies by exactly their extents, so a
bounce cannot re-trigger on the next frame. Breakout uses it for the walls, the
brick grid and the paddle, with the paddle steering by contact offset.

`examples/physics-lab` is a headless check of the core: gravity, a floor
bounce, tunnelling, restitution and a 4,000-step Breakout session. It prints
`passed 12, failed 0`:

```sh
cd examples/physics-lab
sh tests/physics.sh
```

## Rendering

The demo uses the [Nova TUI](../tui/) runtime: a model of `Game` plus a paddle target, key events, a tick that calls `step`, and a view that maps the engine's coordinates onto terminal rows and columns (bricks as blocks, the ball as `●`, the paddle as `▬`). Terminal width is not simulated exactly; the mapping is proportional.

## Limits

- **Breakout physics only.** No sprites, scenes, audio, tilemaps, collision layers or entity components. The solver is AABB-only: no rotation, friction, joints or continuous rotation contacts.
- **Fixed tick.** The simulation is millipixel-integer per 16 ms tick; there is no delta-time and no sub-tick integration.
- **One ball and one paddle.** The engine models exactly the Breakout pieces.
- **Approximate terminal rendering.** The view maps an 800×520 field onto a character grid; it is a demo renderer, not a pixel-accurate one.
- **Desktop only.** The engine is pure Nova and portable, but the included game runs in a terminal through nova-tui.
