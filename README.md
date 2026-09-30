# Alkalab

A powder-sand **chemistry lab** in a browser, written in Rust and compiled to
WebAssembly. Paint sand, water, lava, steel, calcium carbide or a lump of
sodium, then watch the physics and the chemistry fight it out.

* **Sodium dropped in water detonates**, throws out hydrogen and leaves lye behind.
* **Calcium carbide fizzes acetylene** you can set on fire.
* **Lava fuses sand into glass** and melts steel; water quenches it back to stone.
* **Density does the work**: sand sinks, oil floats, mercury sinks through both.
* 32 materials, 40 declarative reaction rules, particles accelerating under gravity.

Everything is simulated in a flat `Vec<Cell>` and rendered straight out of wasm
linear memory into a `Uint8ClampedArray` - no data is copied across the boundary
per frame.

```
cargo test        # the whole test suite, run natively, no browser required
npm run build     # cargo + wasm-pack -> web/pkg
npm run serve     # static server (correct application/wasm MIME type)
```

---

## Quick start

Requirements:

| tool | why | how |
| --- | --- | --- |
| Rust (1.70+) | the whole engine | <https://rustup.rs> |
| `wasm32-unknown-unknown` target | the wasm build | `rustup target add wasm32-unknown-unknown` |
| `wasm-pack` | wasm + JS glue | `cargo install wasm-pack`, or drop a [release](https://rustwasm.github.io/wasm-pack/installer/) into `tools/` |
| Node 18+ (optional) | dev server + headless wasm test | <https://nodejs.org> |

```bash
git clone <this repo> && cd Alkalab
npm run build     # compiles to wasm, writes web/pkg/ (alkalab.wasm + the JS glue)
npm run serve     # http://localhost:8080
```

`web/index.html` imports the ES module `web/pkg/alkalab.js`, which loads
`web/pkg/alkalab.wasm`, so the site has to be served over HTTP (`application/wasm`
matters for `instantiateStreaming`) - opening the file straight from disk will not
work. `scripts/serve.mjs` is a small zero-dependency server that does exactly that.

### On Windows without `wasm-pack` on PATH

`scripts/build.mjs` looks for `wasm-pack` on `PATH` first and then in `tools/`,
so a downloaded release binary is enough:

```powershell
Invoke-WebRequest 'https://github.com/rustwasm/wasm-pack/releases/download/v0.15.0/wasm-pack-v0.15.0-x86_64-pc-windows-msvc.tar.gz' -OutFile tools/wasm-pack.tar.gz
tar -xzf tools/wasm-pack.tar.gz -C tools
npm run build
```

## Scripts

| command | what it does |
| --- | --- |
| `cargo test` | unit + integration tests, run natively (fast, no wasm needed) |
| `cargo clippy --all-targets -- -D warnings` | lint |
| `cargo fmt --all` | format |
| `npm run build` | `scripts/build.mjs`: wasm-pack -> `web/pkg/alkalab.wasm` + glue |
| `npm run bundle` | copies just the runtime files into `dist/` for another host |
| `npm run serve` | static server for `web/` (or `node scripts/serve.mjs dist`) |
| `npm run test:wasm` | loads the built wasm in Node and drives the engine through the shared-memory path |
| `npm run test:dom` | loads `web/main.js` against a DOM stub: palette, pointer painting, RAF loop, keys, log |
| `npm run test:js` | both of the above |
| `npm start` | build, then serve |

---

## Shipping it into a portfolio

Five files are needed at runtime. `npm run bundle` copies exactly those into
`dist/`, preserving the layout:

| file | size | what it is |
| --- | --- | --- |
| `index.html` | 5.9 KB | markup: canvas, palette, reaction log, notes |
| `style.css` | 6.1 KB | the dark lab theme - no external fonts, images or CSS |
| `main.js` | 14 KB | input handling, the RAF loop, `putImageData`, the demos |
| `pkg/alkalab.js` | 12 KB | the wasm-bindgen glue - **required**, the binary cannot be loaded without it |
| `pkg/alkalab.wasm` | 47 KB | the simulation itself |
| | **~85 KB** | the whole game, no dependencies |

`pkg/alkalab.d.ts` and `pkg/alkalab.wasm.d.ts` are TypeScript declarations (dev
only) and the Rust crate never ships. The favicon is an inline data URI, so there
is no sixth file hiding anywhere.

Three rules for wherever you put it:

1. **Keep the layout.** `main.js` imports `./pkg/alkalab.js`, and the glue fetches
   `alkalab.wasm` from its own directory, so keep the pair together.
2. **Serve it over HTTP** with `application/wasm` for the binary. GitHub Pages,
   Netlify, Vercel, S3 and Cloudflare Pages all do this; a bare `file://` open does
   not, and neither does a server that guesses `application/octet-stream`.
3. **ES modules.** The page uses `<script type="module">` and top-level `await`,
   which every current browser supports.

**Inside a bundler (Vite / Next / Astro):** drop `pkg/` into `public/` unchanged
and `import init, { Engine } from '/pkg/alkalab.js'`; the glue resolves its `.wasm`
relative to itself, so nothing else is required. If you would rather control the
fetch (or host the binary elsewhere), pass it in:
`init({ module_or_path: '/assets/alkalab.wasm' })`.

**Embedding into an existing page:** keep the `<canvas id="lab">` and the element
ids `main.js` looks up (`palette`, `selected-note`, `presets`, `log`, `brush`,
`brush-out`, `shape`, `speed`, `boom`, `pause`, `step`, `clear`, `undo`, `save`,
`stat-fps`, `stat-tps`, `stat-frame`, `stat-particles`), or skip the markup entirely
and drive the engine yourself - `web/main.js` is ~400 commented lines, and
`globalThis.alkalab` exposes the live engine for console experiments:

```js
alkalab.engine.paint(40, 40, 6, alkalab.idOf('Sodium'));
alkalab.engine.step_n(60);
alkalab.engine.reactions();
```



### Memory layout

```
Grid (src/grid.rs)

  stride = width + 2                       one cell inert border, filled with Wall
  cells : Vec<Cell>                        (width+2) * (height+2)
  stamp : Vec<u32>                         frame number of the last update

  index(x, y) = (y + 1) * stride + (x + 1)

  Cell { element: u8, variant: u8, life: u16, vel: i8, residue: u8 }   // 6 bytes
```

A `(x, y)` coordinate is pure arithmetic on one flat allocation, so scanning a
row is a linear walk through the cache. The border means every interior cell has
all eight neighbours *inside the buffer*, so neighbour lookups need no bounds
checks, and out-of-range coordinates clamp onto the border instead of panicking -
nothing can escape the world.

### The tick (`World::advance`)

1. **Bottom-up rows.** Falling cells move down into rows that were already
   processed, so a grain can never be updated twice in a frame and teleport to
   the floor.
2. **Alternating column direction.** Even frames scan left to right, odd frames
   right to left, so water has no favourite side and the world has no drift.
3. **Stamps.** Anything that moves on another axis (gases rising, liquids sliding
   sideways) is stamped with the frame number and skipped for the rest of the tick.
4. **Momentum.** A falling cell gains `vel`, so it accelerates from 1 cell/tick to
   3 (the multi-cell move is still one logical update, so ordering holds).

### Rules, not special cases

Each element is one row in a `const PROPS` table (`src/element.rs`): category,
density, colour, lifetime, flammability, burn residue, blast radius, corrosion,
dispersion, buoyancy, heat. Movement dispatches on the *category*:

| category | behaviour |
| --- | --- |
| Powder | falls (accelerating), then slides down-left / down-right at 45°, which is what forms piles |
| Liquid | falls, slides diagonally, searches up to `dispersion` cells sideways for a hole, then levels out one cell at a time |
| Gas | rises (accelerating), bubbles up through liquids, drifts sideways under a ceiling, then decays into its residue |
| Fire | stays on fuel until it is gone, then climbs and flickers; lifetime becomes smoke |
| Solid | never moves; plants grow, everything else reacts through the table |

One density comparison - `can_sink` / `can_rise` - gives sand sinking through
water, oil floating on it, mercury sinking through everything, snow floating, and
hydrogen bubbling up through a lake, with no per-element code.

### Chemistry

`src/reaction.rs` is a table of 40 rows. Each row says: element `a` next to
element `b`, a chance out of 255 per tick, what happens to each side
(`Keep`, `Empty`, `Become(element)`, `Weaken(strength)`, `Wet`), an optional
`Explode { radius, gas }`, and the balanced equation to show in the UI log.

```rust
boom(                                    // 2 Na + 2 H2O -> 2 NaOH + H2
    Element::Sodium, Element::Water, 128, 6, Element::Hydrogen,
    Outcome::Empty, Outcome::Become(Element::SaltWater),
    "2 Na + 2 H2O -> 2 NaOH + H2  (EXPLOSION)",
),
```

Adding a rule is one array entry: the pair lookup is built at first use, it is
registered in both directions automatically, a consumption guard prevents two
reactions eating the same cell in one tick, and the equation appears in the log
for free.

Heat is generic too: every element with `heat > 0` ignites flammable neighbours
(chance = the fuel's `flammable`, wet fuel is inert) and explosive materials
detonate instead of burning - which is how a single spark chains through TNT.

### Rendering: zero copies

Rust owns the pixels:

```rust
pixels: Vec<u8>                          // width * height * 4, [R, G, B, A]
```

and hands the address to JavaScript:

```js
const wasm = await init();
const engine = new Engine(320, 200, seed);
const pixels = new Uint8ClampedArray(wasm.memory.buffer, engine.pixel_ptr, engine.pixel_len);
const imageData = new ImageData(pixels, 320, 200);   // same memory, no copy
ctx.putImageData(imageData, 0, 0);
```

The array *is* wasm memory, so a frame costs one `render()` pass in Rust plus one
`putImageData`. Two details matter:

* the view is rebuilt whenever `wasm.memory.buffer` changes identity, because
  growing the heap detaches the old `ArrayBuffer` (`syncBuffer()` in `web/main.js`);
* air is pre-rendered once into a background buffer and copied in at the start of
  every frame, so translucent steam, smoke and gas can blend over it without
  smearing the previous frame into the current one.

### Per-frame API

Input, the animation loop and the DOM all live in JavaScript; Rust only sees
`paint`, `paint_line`, `paint_rect`, `detonate`, `snapshot`/`undo`, `step_n` and
a handful of getters. `web/main.js` is the whole front end.

---

## Materials

| group | elements |
| --- | --- |
| Powders | Sand, Salt, Ash, Snow, Gunpowder*, Carbide, Lime, Sodium, Potassium, Lithium, Calcium, Magnesium |
| Liquids | Water, Brine, Acid, Oil, Lava, Mercury |
| Solids | Wall (indestructible), Stone, Glass, Wood, Plant, Metal, Ice, TNT* |
| Energy & gases | Fire, Smoke, Steam, Hydrogen*, Acetylene* |

`*` detonates when ignited.

Reactions worth trying: sodium or potassium in water, carbide in water, lava
meeting water or sand, acid on steel, ice on lava, salt on ice, a flame on wet
gunpowder, hydrogen in a sealed pocket, a plant next to water.

## Testing

```bash
cargo test                    # 111 tests, all native, ~2 s
npm run test:wasm             # headless wasm smoke test (needs npm run build first)
```

* **Unit tests** live next to the code (`#[cfg(test)] mod tests`): index maths and
  border behaviour, colour packing and blending, PRNG determinism, the property
  table's invariants (every element has a colour, densities are ordered, fuels
  declare a burn duration, only bedrock is inedible to explosions).
* **Integration tests** (`tests/physics.rs`, `tests/chemistry.rs`) drive the whole
  world and assert on emergent behaviour: a grain falls exactly one cell on its
  first tick and never faster than terminal velocity, piles settle with no
  floating grains, water levels out and drains through a one-cell hole, mercury /
  water / oil sort into density order, snow melts, sodium detonates, potassium
  out-blasts sodium, TNT chains, hydrogen burns back to water, wet TNT refuses to
  detonate, acid burns a hole through plate steel, and explosions never break the
  wall.
* **Invariants and fuzzing** (`tests/invariants.rs`): identical seed and input
  produce a byte-identical world (state hash *and* pixels), undo restores the
  exact state, the pixel buffer is always a full opaque RGBA image, and 120 rounds
  of random painting, stroking and blasting never panic or leak outside the border.
* **The wasm smoke test** (`tests/wasm/smoke.mjs`) loads the real `web/pkg` module
  in Node, wraps linear memory in a `Uint8ClampedArray` exactly like the canvas
  renderer, and checks the shared buffer, the palette JSON, sand fall, the sodium
  reaction log, the boom tool and undo.

Everything is deterministic by construction (own PRNG, integer maths, no hash map
iteration), which is what makes the fuzz and replay tests possible.

## Performance

Measured on the machine this was written on, from the smoke test and
`cargo test`:

| what | number |
| --- | --- |
| wasm binary | ~47 KB (`-O` + LTO + `wasm-opt`) |
| shared pixel buffer | 256 KB for 320x200, zero copies per frame |
| busy world (24 material columns, ~20k active cells) | ~0.5-2 ms per tick, ~450-2000 ticks/s |
| rendering | one pass over 64,000 cells, blended over a pre-rendered background |

So a 60 FPS frame at 1x speed uses a small fraction of the budget, and the speed
slider can run up to 4 ticks per frame. `step_n` renders once at the end, so
sub-ticks do not pay for pixels that would only be overwritten.

## Deploying

`web/` is the complete static site (after `npm run build`). Copy it anywhere -
GitHub Pages, Netlify, S3. `.github/workflows/pages.yml` does it for you: it lints,
runs the native tests, builds the wasm, runs the headless smoke test and publishes
`web/` to Pages on every push to `main`. Enable it once under
**Settings -> Pages -> Source: GitHub Actions**.

## Extending

**A new element** is two edits in `src/element.rs`:

```rust
// 1. append to the enum (ids are the wasm API, so never reorder)
pub enum Element { /* ... */ Acetylene, Thermite }

// 2. append the matching row to the property table
Props {
    speckle: 8,
    heat: 200,
    ..corrodes(burns(base(
        "Thermite", "Fe2O3+Al", Category::Powder, 190,
        rgb(120, 70, 60),
        "Burns hot enough to melt steel.",
    ), 255, 300, Element::Metal))
},
```

Ranked by density, it sinks; given `heat` it ignites things; given `burns` it can
be ignited; the palette, tooltips, the reaction log and the tests pick it up
automatically (including the "every element has a colour / a name / a formula"
invariants).

**A new reaction** is one entry in `REACTIONS` in `src/reaction.rs` - the pair
lookup, the reverse direction and the UI log line are all derived from it.

## Project layout

```
src/
  lib.rs         crate docs, module wiring, re-exports
  world.rs       World: state + pixels + the tick loop + painting tools
  element.rs     Element enum, Category, the const Props table
  cell.rs        Cell: element, variant, life, momentum, residue
  grid.rs        flat bordered Vec<Cell> + per-frame stamps
  physics.rs     movement per category, ignition
  reaction.rs    the reaction table, pair lookup, blast effects
  render.rs      grid -> shared RGBA pixel buffer
  color.rs       packed colours, blending, positional noise
  rng.rs         deterministic xorshift32
  wasm.rs        the wasm-bindgen surface (wasm32 only)
web/
  index.html     the page: canvas, palette, log, notes
  main.js        input, RAF loop, ImageData, presets
  style.css      dark lab theme, no external assets
tests/
  physics.rs     movement and density behaviour
  chemistry.rs   the set-piece reactions
  invariants.rs  determinism, undo, fuzzing
  wasm/smoke.mjs headless wasm test
scripts/
  build.mjs      the build: wasm-pack -> web/pkg, binary renamed to alkalab.wasm
  build.ps1      Windows convenience wrapper around build.mjs
  bundle.mjs     copies the five runtime files into dist/ for another host
  serve.mjs      zero-dependency static server with the right MIME types
dist/            generated by `npm run bundle`: the deployable site, ~85 KB
```

The wasm binary is `web/pkg/alkalab.wasm` (wasm-bindgen names it `<out-name>_bg.wasm`,
so `scripts/build.mjs` renames it after the build and points the generated glue at
the new name - the reason a plain `wasm-pack build` on its own produces
`alkalab_bg.wasm`).

## Credits

Inspired by the falling-sand games of the 2000s and 2010s - Dan-Ball's *Powder
Game* in particular - and by the chemistry of the periodic table's more
excitable metals. Built with [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen).

MIT licensed.

