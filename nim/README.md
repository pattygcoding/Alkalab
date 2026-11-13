# Alkalab - Nim + WebAssembly port

A self-contained Nim rewrite of the Alkalab falling-sand chemistry lab. The
engine is split into focused modules under `src/alkalab/`, runs natively for
tests, and compiles to a standalone `wasm32-unknown-unknown` module that the
existing browser front end drives through a small generated ES module.

The original Rust project in the repository root is untouched. This folder is
the Nim implementation. The build here is what the Arcade-PG-Website `/alkalab`
page serves, copied into `public/wasm/alkalab.js` and `alkalab.wasm`.

## What is implemented

* The complete 40-element catalogue with density, colour, flammability,
  corrosion, lifetime and residue properties, including fun additions like
  thermite (burns into lava), methane (explosive gas), coal, dry ice,
  liquid nitrogen (freezes water), fireworks, copper and acid-proof gold.
* The full physics pass: powders fall and pile, liquids spread and level, gases
  rise, fire spreads, solids stay put.
* The declarative reaction table (water + sodium/potassium/lithium/calcium,
  carbide + water -> acetylene, acid + metal/carbonate, lava + sand -> glass,
  ice + salt/heat, TNT/gunpowder explosions, and the rest).
* The zero-copy RGBA renderer, deterministic xorshift32 RNG, reaction log and
  the 16-snapshot undo stack.
* A wasm C ABI (`engine_*` exports) plus a generated JS `Engine` class.
* A native Nim test suite and a Node smoke test for the wasm build.

## Requirements

* Nim 2.2 or newer
* clang with the `wasm32-unknown-unknown` target and `wasm-ld`
* Node 18 or newer (for the smoke test and the static server)

On Windows the Nim distribution ships clang; make sure its `bin` directory is
on `PATH`.

## Build

```powershell
# from the repository root
node nim/build.mjs
```

or, from this folder:

```powershell
.\build.ps1
```

The build produces:

| Path | Purpose |
| --- | --- |
| `src/alkalab.wasm` | raw standalone module |
| `web/pkg/alkalab.wasm` | module served to the page |
| `web/pkg/alkalab.js` | generated ES module (`init`, `Engine`) |
| `web/index.html`, `web/main.js`, `web/style.css` | copied browser front end |

Serve it with the repository's zero-dependency server:

```powershell
node scripts/serve.mjs nim/web
# open http://localhost:8080/
```

## Tests

Native engine tests (no browser required):

```powershell
nim c -r --hints:off --warnings:off nim/tests/alkalab_tests.nim
```

Wasm + JS glue smoke test:

```powershell
node nim/build.mjs
node nim/tests/wasm_smoke.mjs
```

## Layout

```
nim/
  build.mjs              # compile wasm + generate glue + copy web assets
  build.ps1              # Windows wrapper
  src/
    alkalab.nim          # public re-export of the modules below
    alkalab/
      color.nim          # packed RGBA helpers, blending, positional noise
      rng.nim            # deterministic xorshift32 PRNG
      element.nim        # 40-element catalogue, properties and categories
      celltype.nim       # single-cell state (element, variant, life, vel)
      grid.nim           # flat bordered buffer + per-frame update stamp
      neighbours.nim     # Moore/von Neumann neighbour offset tables
      render.nim         # state buffer -> packed RGBA pixel buffer
      physics.nim        # movement per category, plant growth, ignition
      reaction.nim       # declarative reaction table and engine
      json.nim           # catalogue + reaction-log JSON builders
      world_types.nim    # World/ReactionLog types, undo, painting tools
      world.nim          # world construction and the per-tick loop
      wasm_api.nim       # `engine_*` C ABI exports
    wasm_libc.c          # tiny memcpy/memset/malloc over memory.grow
    wasm_include/        # minimal libc headers for the standalone target
    panicoverride.nim    # standalone panic hook
  tests/
    alkalab_tests.nim    # native unittest suite
    wasm_smoke.mjs       # instantiates the module and drives the Engine class
  web/                   # generated/copied browser front end
```

## How the wasm build works

The standalone target has no operating system and no libc. The engine therefore
ships:

* `wasm_include/{string,stdlib,math,stdio}.h` - just enough declarations for
  Nim's generated C.
* `wasm_libc.c` - `memcpy`/`memset`/`memmove`/`memcmp` plus a bump `malloc`
  that grows wasm linear memory from `__heap_base`. `-d:useMalloc` routes Nim's
  allocations through it.
* `panicoverride.nim` - turns a Nim panic into `__builtin_unreachable()`.

The build links with `-nostdlib -Wl,--no-entry -Wl,--export-all
-Wl,--export-memory` and starts with 96 MiB of memory (256 MiB maximum). The
engine allocates one `WorldObj`; its 64 KiB JSON scratch buffer and the pixel
buffer are read directly from JS.

`newWorld` uses the same `new` under `mm:arc` for native and wasm builds, so a
single source path runs everywhere; `-d:useMalloc` routes allocations to the
custom allocator only for the standalone target.

## Browser API

`init(options?)` instantiates the module. It accepts an `ArrayBuffer`, a typed
array, a URL/string, or `{ module_or_path }`; with no argument it fetches
`./alkalab.wasm` next to the glue. It returns the wasm exports with `memory`.

`new Engine(width, height, seed)` creates the world. The class exposes:

```js
engine.width, engine.height, engine.frame
engine.pixel_ptr, engine.pixel_len
engine.particle_count, engine.can_undo, engine.hash
engine.step(), engine.step_n(ticks)
engine.paint(x, y, radius, elementId)
engine.paint_line(x0, y0, x1, y1, radius, elementId)
engine.paint_rect(x0, y0, x1, y1, elementId)
engine.detonate(x, y, radius)
engine.clear(), engine.snapshot(), engine.undo()
engine.element_at(x, y), engine.count_of(elementId)
engine.element_catalog(), engine.reactions()
```

`element_catalog()` and `reactions()` return JSON strings decoded from the
engine's scratch buffer; copy/parse them immediately if you need to keep them.
