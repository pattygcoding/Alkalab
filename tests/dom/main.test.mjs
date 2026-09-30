// Front-end integration test.
//
// main.js is the real browser entry point: it builds the palette, wires up the
// pointer and keyboard handlers and runs the requestAnimationFrame loop. This
// test loads it in Node against a minimal DOM stub, so the whole front end is
// exercised without a browser: palette generation from the Rust catalogue,
// painting through synthetic pointer events, the RAF loop calling putImageData
// with the shared buffer, the HUD and the reaction log updating, keyboard
// shortcuts, presets and undo.
//
//   node tests/dom/main.test.mjs
//
// Run `npm run build` first.

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

// main.js calls `init()` with no argument, which makes the wasm-bindgen glue
// fetch `alkalab.wasm` relative to its own URL. Node's fetch does not do
// file:// URLs, so serve the bytes from disk - with the same MIME type the dev
// server sends, so the glue can use WebAssembly.instantiateStreaming.
globalThis.fetch = async (url) => {
  const bytes = await readFile(fileURLToPath(url));
  return new Response(bytes, { headers: { 'content-type': 'application/wasm' } });
};

// ---------------------------------------------------------------------------
// a very small DOM
// ---------------------------------------------------------------------------
class ClassList {
  constructor() {
    this.set = new Set();
  }
  add(...names) {
    names.forEach((n) => this.set.add(n));
  }
  remove(...names) {
    names.forEach((n) => this.set.delete(n));
  }
  contains(name) {
    return this.set.has(name);
  }
  toggle(name, force) {
    const want = force ?? !this.set.has(name);
    if (want) this.set.add(name);
    else this.set.delete(name);
    return want;
  }
}

class StubElement {
  constructor(tag = 'div') {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.listeners = new Map();
    this.classList = new ClassList();
    this.style = {};
    this.dataset = {};
    this.textContent = '';
    this.title = '';
    this.value = '';
    this.type = '';
    this.className = '';
  }
  addEventListener(type, handler) {
    const list = this.listeners.get(type) ?? [];
    list.push(handler);
    this.listeners.set(type, list);
  }
  dispatch(type, event = {}) {
    const list = this.listeners.get(type) ?? [];
    for (const handler of list) {
      handler({ target: this, preventDefault() {}, ...event });
    }
    return list.length;
  }
  append(...nodes) {
    this.children.push(...nodes);
  }
  appendChild(node) {
    this.children.push(node);
    return node;
  }
  replaceChildren(...nodes) {
    this.children = nodes;
  }
  /** Depth first text content, for assertions. */
  text() {
    return [this.textContent, ...this.children.map((c) => c.text())].join(' ');
  }
}

class HTMLInputElement extends StubElement {}
class HTMLSelectElement extends StubElement {}

class StubContext {
  constructor() {
    this.putImageDataCalls = 0;
    this.lastImageData = null;
    this.imageSmoothingEnabled = true;
  }
  putImageData(imageData, x, y) {
    this.putImageDataCalls += 1;
    this.lastImageData = imageData;
    this.at = { x, y };
  }
}

class StubCanvas extends StubElement {
  constructor(width, height) {
    super('canvas');
    this.width = width;
    this.height = height;
    this.ctx = new StubContext();
    this.captured = null;
  }
  getContext() {
    return this.ctx;
  }
  getBoundingClientRect() {
    // One CSS pixel per cell, so event coordinates map straight onto cells.
    return { left: 0, top: 0, width: this.width, height: this.height };
  }
  setPointerCapture(id) {
    this.captured = id;
  }
  toBlob(callback) {
    callback({ size: 1 });
  }
}

class StubImageData {
  constructor(data, width, height) {
    this.data = data;
    this.width = width;
    this.height = height;
  }
}

const elements = new Map();
const byId = (id) => {
  if (!elements.has(id)) {
    let element;
    if (id === 'lab') element = new StubCanvas(320, 200);
    else if (id === 'brush' || id === 'speed') element = new HTMLInputElement('input');
    else if (id === 'shape') element = new HTMLSelectElement('select');
    else element = new StubElement('div');
    element.id = id;
    elements.set(id, element);
  }
  return elements.get(id);
};
byId('brush').value = '4';
byId('speed').value = '1';
byId('shape').value = 'circle';

const windowListeners = new Map();
const stubWindow = {
  addEventListener(type, handler) {
    const list = windowListeners.get(type) ?? [];
    list.push(handler);
    windowListeners.set(type, list);
  },
  dispatch(type, event = {}) {
    for (const handler of windowListeners.get(type) ?? []) {
      handler({ preventDefault() {}, ...event });
    }
  },
};

const frameQueue = [];
let clock = 0;

globalThis.document = {
  getElementById: byId,
  createElement: (tag) => new StubElement(tag),
};
globalThis.window = stubWindow;
globalThis.ImageData = StubImageData;
globalThis.HTMLInputElement = HTMLInputElement;
globalThis.HTMLSelectElement = HTMLSelectElement;
globalThis.requestAnimationFrame = (callback) => {
  frameQueue.push(callback);
  return frameQueue.length;
};

function runFrames(count) {
  for (let i = 0; i < count; i += 1) {
    const callback = frameQueue.shift();
    assert.ok(callback, 'the loop should keep requesting frames');
    clock += 16.7;
    callback(clock);
  }
}

// ---------------------------------------------------------------------------
// load the real front end
// ---------------------------------------------------------------------------
await import('../../web/main.js');

const { engine, catalog } = globalThis.alkalab;
const canvas = byId('lab');
const palette = byId('palette');
const presets = byId('presets');
const idOf = (name) => catalog.find((entry) => entry.name === name).id;
const SODIUM = idOf('Sodium');

// --- palette -----------------------------------------------------------------
assert.equal(palette.children.length, 31, 'every element except air gets a button');
const buttonFor = (name) => {
  const button = palette.children.find((child) => child.text().includes(name));
  assert.ok(button, `no palette button for ${name}`);
  return button;
};
assert.ok(palette.children.every((button) => button.title.length > 10));
console.log(`palette built from the Rust catalogue: ${palette.children.length} elements`);

// --- the loop renders through the shared buffer ------------------------------
runFrames(3);
assert.equal(canvas.ctx.putImageDataCalls, 3, 'one blit per frame');
const imageData = canvas.ctx.lastImageData;
assert.equal(imageData.width, 320);
assert.equal(imageData.height, 200);
assert.equal(imageData.data.length, 320 * 200 * 4);
assert.ok(imageData.data instanceof Uint8ClampedArray);
assert.equal(imageData.data.byteLength, engine.pixel_len);
console.log('the canvas is fed by a Uint8ClampedArray view of wasm memory');

// --- painting with synthetic pointer events ---------------------------------
buttonFor('Sodium').dispatch('click');
assert.ok(buttonFor('Sodium').classList.contains('selected'));

const pointer = (type, x, y, extra = {}) =>
  canvas.dispatch(type, {
    clientX: x,
    clientY: y,
    button: 0,
    buttons: 1,
    pointerId: 1,
    ...extra,
  });

const sodiumId = SODIUM;

pointer('pointerdown', 100, 10);
pointer('pointermove', 140, 10);
pointer('pointermove', 220, 10);
pointer('pointerup', 220, 10, { buttons: 0 });

const painted = engine.count_of(sodiumId);
assert.ok(painted > 0, 'a drag should have painted sodium across the whole stroke');

// The whole stroke is a single undoable step.
assert.ok(engine.can_undo, 'the stroke pushed a snapshot');
assert.ok(engine.undo());
assert.equal(engine.count_of(sodiumId), 0, 'undo removes the whole stroke');
console.log(`stroke painted ${painted} cells, one undo removed it`);

// --- right click erases -------------------------------------------------------
buttonFor('Sand').dispatch('click');
pointer('pointerdown', 160, 100, { button: 2, buttons: 2 });
pointer('pointerup', 160, 100, { buttons: 0 });
assert.equal(engine.element_at(160, 100), 0, 'right click erases');
console.log('right click erases');

// --- keyboard shortcuts -------------------------------------------------------
stubWindow.dispatch('keydown', { key: '7', ctrlKey: false, target: stubWindow });
assert.ok(buttonFor('Sodium').classList.contains('selected'), '7 selects sodium');
stubWindow.dispatch('keydown', { key: ' ', ctrlKey: false, target: stubWindow });
assert.equal(byId('pause').textContent, 'Play', 'space pauses');
stubWindow.dispatch('keydown', { key: ' ', ctrlKey: false, target: stubWindow });
assert.equal(byId('pause').textContent, 'Pause', 'space resumes');
console.log('keyboard shortcuts work');

// --- presets, the HUD and the reaction log ------------------------------------
assert.equal(presets.children.length, 6, 'six demo scenes');
presets.children[0].dispatch('click'); // "Sodium vs water"
assert.ok(engine.particle_count > 0, 'the preset built a scene');

let logged = false;
for (let i = 0; i < 40 && !logged; i += 1) {
  runFrames(2);
  logged = byId('log').text().includes('2 Na + 2 H2O');
}
assert.ok(logged, 'the reaction log panel shows the sodium equation');
assert.match(byId('stat-frame').textContent, /^\d+$/);
assert.notEqual(byId('stat-particles').textContent, '0');
console.log(`reaction log rendered: ${byId('log').children[0].textContent}`);

console.log('\nall DOM tests passed');

