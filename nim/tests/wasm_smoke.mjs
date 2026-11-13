// Smoke test for the generated wasm module + JS glue.
//
//   node nim/build.mjs && node nim/tests/wasm_smoke.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import init, { Engine } from '../web/pkg/alkalab.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const wasmPath = path.join(here, '..', 'web', 'pkg', 'alkalab.wasm');
const wasm = await init({ module_or_path: fs.readFileSync(wasmPath) });

const engine = new Engine(64, 48, 1);
const catalog = JSON.parse(engine.element_catalog());
const byName = new Map(catalog.map((entry) => [entry.name, entry]));
const idOf = (name) => byName.get(name).id;

assert.equal(catalog.length, 40);
assert.equal(catalog[0].name, 'Air');
assert.equal(catalog[31].name, 'Acetylene');
assert.equal(catalog[39].name, 'Gold');
assert.ok(byName.has('Thermite'));
assert.ok(byName.has('Liquid Nitrogen'));
assert.equal(catalog[0].formula, 'N2+O2');
assert.equal(engine.width, 64);
assert.equal(engine.height, 48);
assert.equal(engine.pixel_len, 64 * 48 * 4);
assert.equal(engine.frame, 0);

// Paint a sand column and let it fall to the floor.
engine.paint(32, 2, 2, idOf('Sand'));
assert.ok(engine.count_of(idOf('Sand')) > 0);
engine.step_n(60);
assert.equal(engine.element_at(32, 47), idOf('Sand'));

// Water spreads without being destroyed.
engine.paint_rect(8, 20, 56, 40, idOf('Water'));
const water = engine.count_of(idOf('Water'));
engine.step_n(20);
assert.equal(engine.count_of(idOf('Water')), water);

// Sodium + water must log the classic reaction.
engine.paint(32, 20, 2, idOf('Sodium'));
let sawSodium = false;
for (let i = 0; i < 240; i += 1) {
  engine.step();
  if (!sawSodium && engine.reactions().includes('2 Na + 2 H2O')) sawSodium = true;
}
assert.equal(engine.count_of(idOf('Sodium')), 0);
assert.ok(sawSodium);

// The undo stack works across clear().
const before = engine.hash;
engine.clear();
assert.equal(engine.particle_count, 0);
assert.ok(engine.can_undo);
assert.ok(engine.undo());
assert.equal(engine.hash, before);

// The pixel buffer is opaque RGBA and lives in wasm memory.
const pixels = new Uint8Array(wasm.memory.buffer, engine.pixel_ptr, engine.pixel_len);
assert.equal(pixels.length, engine.pixel_len);
for (let i = 3; i < pixels.length; i += 4) {
  assert.equal(pixels[i], 255);
}

// Catalog JSON stays valid after the simulation has run.
const after = JSON.parse(engine.element_catalog());
assert.equal(after.length, 40);
assert.equal(after[9].name, 'Sand');
assert.equal(after[39].name, 'Gold');

// New elements react. Liquid nitrogen freezes water into ice.
const cryo = new Engine(64, 48, 7);
cryo.paint_rect(8, 20, 56, 40, idOf('Water'));
cryo.paint(32, 18, 1, idOf('Liquid Nitrogen'));
cryo.step_n(40);
assert.ok(cryo.count_of(idOf('Ice')) > 0);
cryo.free();

// Gold resists acid while copper does not.
const metals = new Engine(64, 48, 8);
metals.paint_rect(8, 30, 20, 40, idOf('Gold'));
metals.paint_rect(8, 20, 20, 28, idOf('Acid'));
const gold = metals.count_of(idOf('Gold'));
metals.step_n(240);
assert.equal(metals.count_of(idOf('Gold')), gold);
metals.free();

console.log('wasm smoke test passed:', {
  elements: catalog.length,
  particles: engine.particle_count,
  frame: engine.frame,
  pixelBytes: engine.pixel_len,
});
