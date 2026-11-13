# Alkalab

A powder-sand **chemistry lab** in a browser, written in Nim and compiled to
standalone WebAssembly. Paint sand, water, lava, steel, calcium carbide or a lump
of sodium, then watch the physics and the chemistry fight it out.

* **Sodium dropped in water detonates**, throws out hydrogen and leaves lye behind.
* **Calcium carbide fizzes acetylene** you can set on fire.
* **Lava fuses sand into glass** and melts steel; water quenches it back to stone.
* **Density does the work**: sand sinks, oil floats, mercury sinks through both.
* 40 materials, 49 declarative reaction rules, particles accelerating under gravity.

Everything is simulated in a flat `seq[Cell]` and rendered straight out of wasm
linear memory into a `Uint8ClampedArray` - no data is copied across the boundary
per frame.

The Rust + wasm-bindgen implementation this was ported from is preserved at
[**alkalab-legacy**](https://github.com/pattygcoding/alkalab-legacy).

```sh
npm run test      # native Nim test suite, no browser required
npm run build     # nim -> nim/web/pkg (alkalab.wasm + ES module glue)
npm run serve     # static server for nim/web
```

---

## Quick start

Requirements:

| tool | why | how |
| --- | --- | --- |
| Nim 2.2+ | the whole engine | <https://nim-lang.org/install.html> |
| clang with the `wasm32-unknown-unknown` target and `wasm-ld` | the standalone wasm build | install LLVM/clang + lld; on Windows make sure the toolchain's `bin` is on `PATH` |
| Node 18+ (optional) | build script, headless test and dev server | <https://nodejs.org> |

```sh
git clone <this repo> && cd Alkalab
npm run build     # compiles to wasm, writes nim/web/pkg/ (alkalab.wasm + the JS glue)
npm run serve     # http://localhost:8080
```

`nim/web/index.html` imports the ES module `nim/web/pkg/alkalab.js`, which loads
`nim/web/pkg/alkalab.wasm`, so the site has to be served over HTTP - opening the
file straight from disk will not work. `scripts/serve.mjs` is a small
zero-dependency server.

## Scripts

| command | what it does |
| --- | --- |
| `npm run build` | `nim/build.mjs`: Nim -> wasm, generate the ES module glue, copy `web/` into `nim/web/` |
| `npm run serve` | static server for `nim/web/` (port 8080) |
| `npm start` | build, then serve |
| `npm test` | native Nim test suite (`nim c -r ... nim/tests/alkalab_tests.nim`) |
| `npm run test:wasm` | loads the built wasm in Node and drives the engine through the shared-memory path |
| `npm run test:js` | alias for `npm run test:wasm` |

## Shipping it into a portfolio

The complete static site is `nim/web/` after a build:

| file | what it is |
| --- | --- |
| `index.html` | markup: canvas, palette, reaction log, notes |
| `style.css` | the dark lab theme - no external fonts, images or CSS |
| `main.js` | input, RAF loop, ImageData, presets |
| `pkg/alkalab.js` | generated ES module (`init`, `Engine`) |
| `pkg/alkalab.wasm` | the engine |

`nim/build.mjs` copies `web/index.html`, `web/main.js` and `web/style.css` into
`nim/web/`, then writes the generated glue next to the wasm binary.

## Deploying

`.github/workflows/pages.yml` installs Nim and the wasm toolchain, runs the native
tests, builds the wasm, runs the headless smoke test and publishes `nim/web` to
GitHub Pages on every push to `main`. Enable it once under
**Settings -> Pages -> Source: GitHub Actions**.

## Extending

**A new element** is two edits in `nim/src/alkalab/element.nim`: append to the enum
(ids are the wasm API, so never reorder) and append the matching row to the
property table. The palette, tooltips, reaction log and tests pick it up
automatically.

**A new reaction** is one entry in `REACTIONS` in `nim/src/alkalab/reaction.nim` -
the pair lookup, the reverse direction and the UI log line are all derived from it.

## Project layout

```
nim/
  build.mjs              # compile wasm + generate glue + copy web assets
  build.ps1              # Windows wrapper
  src/
    alkalab.nim          # public re-export of the modules below
    alkalab/             # color, rng, element, grid, physics, reaction, render, ...
    wasm_libc.c          # tiny memcpy/memset/malloc over memory.grow
    wasm_include/        # minimal libc headers for the standalone target
    panicoverride.nim    # standalone panic hook
  tests/
    alkalab_tests.nim    # native unittest suite
    wasm_smoke.mjs       # instantiates the module and drives the Engine class
  web/                   # generated/copied browser front end
web/
  index.html             # the shared front end copied by nim/build.mjs
  main.js
  style.css
scripts/
  serve.mjs              # zero-dependency static server with the right MIME types
```

The wasm binary is `nim/web/pkg/alkalab.wasm`; `nim/build.mjs` also writes the
small ES module `nim/web/pkg/alkalab.js` that wraps the raw C-ABI exports in an
`Engine` class.

See [`nim/README.md`](nim/README.md) for the module layout and browser API.

## Credits

Inspired by the falling-sand games of the 2000s and 2010s - Dan-Ball's *Powder
Game* in particular - and by the chemistry of the periodic table's more
excitable metals. Built with Nim's standalone `wasm32-unknown-unknown` target;
the earlier Rust + `wasm-bindgen` implementation lives at
[alkalab-legacy](https://github.com/pattygcoding/alkalab-legacy).

MIT licensed.
