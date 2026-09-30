// Alkalab - the browser half.
//
// JavaScript owns the DOM, the input events and the requestAnimationFrame loop.
// Rust owns the state buffer, the physics and the pixels. The only thing that
// crosses the boundary per frame is a pointer: the RGBA buffer lives inside wasm
// linear memory and is handed straight to `putImageData`.

import init, { Engine } from './pkg/alkalab.js';

const WIDTH = 320;
const HEIGHT = 200;

// ---------------------------------------------------------------------------
// boot
// ---------------------------------------------------------------------------
const wasm = await init();
const engine = new Engine(WIDTH, HEIGHT, (Math.random() * 0xffffffff) >>> 0);

const catalog = JSON.parse(engine.element_catalog());
const byName = new Map(catalog.map((entry) => [entry.name, entry]));
const idOf = (name) => byName.get(name).id;

const canvas = document.getElementById('lab');
const ctx = canvas.getContext('2d', { alpha: false });

// The wasm heap can grow (allocating any String from Rust may move it), and
// growing detaches the old ArrayBuffer, so rebuild the view when that happens.
let pixelBuffer = null;
let pixels = null;
let imageData = null;

function syncBuffer() {
  const buffer = wasm.memory.buffer;
  if (buffer !== pixelBuffer) {
    pixelBuffer = buffer;
    pixels = new Uint8ClampedArray(buffer, engine.pixel_ptr, engine.pixel_len);
    imageData = new ImageData(pixels, WIDTH, HEIGHT);
  }
}

// ---------------------------------------------------------------------------
// state
// ---------------------------------------------------------------------------
let selected = byName.get('Sand');
let brushRadius = 4;
let brushShape = 'circle';
let tool = 'paint'; // 'paint' | 'boom'
let running = true;
let speed = 1;

// ---------------------------------------------------------------------------
// palette
// ---------------------------------------------------------------------------
const paletteEl = document.getElementById('palette');
const selectedNote = document.getElementById('selected-note');
const elementButtons = new Map();

for (const entry of catalog) {
  if (entry.name === 'Air') continue; // air is what right-click erases with
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'element';
  button.title = `${entry.name} (${entry.formula}) - ${entry.desc}`;
  const swatch = document.createElement('span');
  swatch.className = 'swatch';
  swatch.style.background = entry.color;
  const label = document.createElement('span');
  label.textContent = entry.name;
  const formula = document.createElement('span');
  formula.className = 'formula';
  formula.textContent = entry.formula;
  button.append(swatch, label, formula);
  button.addEventListener('click', () => selectElement(entry));
  paletteEl.append(button);
  elementButtons.set(entry.id, button);
}

function selectElement(entry) {
  selected = entry;
  tool = 'paint';
  boomButton.classList.remove('active');
  for (const [id, button] of elementButtons) {
    button.classList.toggle('selected', id === entry.id);
  }
  selectedNote.textContent = `${entry.name} - ${entry.formula}`;
}

// ---------------------------------------------------------------------------
// tools
// ---------------------------------------------------------------------------
const brushInput = document.getElementById('brush');
const brushOutput = document.getElementById('brush-out');
const shapeInput = document.getElementById('shape');
const speedInput = document.getElementById('speed');
const pauseButton = document.getElementById('pause');
const boomButton = document.getElementById('boom');

brushInput.addEventListener('input', () => {
  brushRadius = Number(brushInput.value);
  brushOutput.value = String(brushRadius);
});
shapeInput.addEventListener('change', () => {
  brushShape = shapeInput.value;
});
speedInput.addEventListener('change', () => {
  speed = Number(speedInput.value);
  document.getElementById('stat-tps').textContent = String(speed);
});
pauseButton.addEventListener('click', toggleRunning);
boomButton.addEventListener('click', () => {
  tool = tool === 'boom' ? 'paint' : 'boom';
  boomButton.classList.toggle('active', tool === 'boom');
});
document.getElementById('step').addEventListener('click', () => {
  setRunning(false);
  engine.step();
});
document.getElementById('clear').addEventListener('click', () => engine.clear());
document.getElementById('undo').addEventListener('click', () => engine.undo());
document.getElementById('save').addEventListener('click', () => {
  canvas.toBlob((blob) => {
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = `alkalab-${Date.now()}.png`;
    link.click();
    URL.revokeObjectURL(link.href);
  });
});

function setRunning(next) {
  running = next;
  pauseButton.textContent = running ? 'Pause' : 'Play';
  pauseButton.classList.toggle('active', !running);
}
function toggleRunning() {
  setRunning(!running);
}

// ---------------------------------------------------------------------------
// painting
// ---------------------------------------------------------------------------
let painting = false;
let isErasing = false;
let lastCell = null;

function cellFromEvent(event) {
  const rect = canvas.getBoundingClientRect();
  const x = ((event.clientX - rect.left) / rect.width) * WIDTH;
  const y = ((event.clientY - rect.top) / rect.height) * HEIGHT;
  return { x: Math.round(x), y: Math.round(y) };
}

function paintAt(x, y) {
  const elementId = isErasing ? 0 : selected.id;
  if (tool === 'boom' && !isErasing) {
    engine.detonate(x, y, brushRadius + 3);
    return;
  }
  switch (brushShape) {
    case 'square':
      engine.paint_rect(
        x - brushRadius,
        y - brushRadius,
        x + brushRadius,
        y + brushRadius,
        elementId,
      );
      break;
    case 'spray':
      for (let i = 0; i < 10; i += 1) {
        const spread = brushRadius * 1.6;
        engine.paint(
          Math.round(x + (Math.random() - 0.5) * spread * 2),
          Math.round(y + (Math.random() - 0.5) * spread * 2),
          Math.max(0, Math.round(brushRadius / 2)),
          elementId,
        );
      }
      break;
    default:
      engine.paint(x, y, brushRadius, elementId);
  }
}

// Fast drags deliver sparse pointer events: walk the whole segment so a stroke
// never has gaps in it.
function strokeTo(x, y) {
  if (lastCell) {
    const dx = x - lastCell.x;
    const dy = y - lastCell.y;
    const steps = Math.max(Math.abs(dx), Math.abs(dy), 1);
    for (let i = 1; i <= steps; i += 1) {
      paintAt(Math.round(lastCell.x + (dx * i) / steps), Math.round(lastCell.y + (dy * i) / steps));
    }
  } else {
    paintAt(x, y);
  }
  lastCell = { x, y };
}

canvas.addEventListener('pointerdown', (event) => {
  canvas.setPointerCapture(event.pointerId);
  painting = true;
  isErasing = event.button === 2 || (event.buttons & 2) === 2;
  lastCell = null;
  // One snapshot per stroke, so Ctrl+Z undoes a whole brush stroke.
  engine.snapshot();
  const { x, y } = cellFromEvent(event);
  strokeTo(x, y);
  event.preventDefault();
});

canvas.addEventListener('pointermove', (event) => {
  if (!painting) return;
  const { x, y } = cellFromEvent(event);
  strokeTo(x, y);
});

const stopPainting = () => {
  painting = false;
  lastCell = null;
};
canvas.addEventListener('pointerup', stopPainting);
canvas.addEventListener('pointercancel', stopPainting);
canvas.addEventListener('contextmenu', (event) => event.preventDefault());
canvas.addEventListener(
  'wheel',
  (event) => {
    event.preventDefault();
    brushRadius = Math.max(0, Math.min(24, brushRadius + (event.deltaY < 0 ? 1 : -1)));
    brushInput.value = String(brushRadius);
    brushOutput.value = String(brushRadius);
  },
  { passive: false },
);

// ---------------------------------------------------------------------------
// keyboard
// ---------------------------------------------------------------------------
const HOTKEYS = ['Sand', 'Water', 'Stone', 'Wood', 'Fire', 'Oil', 'Sodium', 'Lava', 'Acid', 'Tnt'];

window.addEventListener('keydown', (event) => {
  if (event.ctrlKey && event.key.toLowerCase() === 'z') {
    engine.undo();
    event.preventDefault();
    return;
  }
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;

  const key = event.key.toLowerCase();
  if (key === ' ') {
    toggleRunning();
  } else if (key === 's') {
    setRunning(false);
    engine.step();
  } else if (key === 'c') {
    engine.clear();
  } else if (key === 'x') {
    boomButton.click();
  } else if (key === 'e') {
    selectElement(byName.get('Air'));
  } else if (key >= '1' && key <= '9') {
    selectElement(byName.get(HOTKEYS[Number(key) - 1]));
  } else if (key === '0') {
    selectElement(byName.get(HOTKEYS[9]));
  } else {
    return;
  }
  event.preventDefault();
});

// ---------------------------------------------------------------------------
// the loop
// ---------------------------------------------------------------------------
const statFps = document.getElementById('stat-fps');
const statFrame = document.getElementById('stat-frame');
const statParticles = document.getElementById('stat-particles');
const logEl = document.getElementById('log');
let lastLog = null;
let fps = 0;
let lastTime = performance.now();
let hudCountdown = 0;

function updateLog() {
  const json = engine.reactions();
  if (json === lastLog) return;
  lastLog = json;
  const entries = JSON.parse(json);
  if (entries.length === 0) {
    logEl.innerHTML = '<li class="empty">Nothing has reacted yet.</li>';
    return;
  }
  logEl.replaceChildren(
    ...entries.map((entry) => {
      const item = document.createElement('li');
      item.textContent = entry.equation;
      if (entry.count > 1) {
        const badge = document.createElement('b');
        badge.textContent = ` x${entry.count}`;
        item.append(badge);
      }
      return item;
    }),
  );
}

function frame(now) {
  const dt = now - lastTime;
  lastTime = now;
  if (dt > 0) fps = fps === 0 ? 1000 / dt : fps * 0.9 + (1000 / dt) * 0.1;

  if (running) engine.step_n(speed);

  syncBuffer();
  ctx.putImageData(imageData, 0, 0);
  updateLog();

  if ((hudCountdown -= 1) <= 0) {
    hudCountdown = 10;
    statFps.textContent = fps.toFixed(0);
    statFrame.textContent = String(engine.frame);
    statParticles.textContent = String(engine.particle_count);
  }

  requestAnimationFrame(frame);
}

// ---------------------------------------------------------------------------
// demo scenes
// ---------------------------------------------------------------------------
const presets = [
  {
    name: 'Sodium vs water',
    build() {
      engine.paint_rect(20, 150, 300, 196, idOf('Water'));
      engine.paint(60, 120, 3, idOf('Sodium'));
      engine.paint(160, 118, 3, idOf('Sodium'));
      engine.paint(260, 120, 3, idOf('Sodium'));
      engine.paint(110, 130, 2, idOf('Potassium'));
      engine.paint(210, 130, 2, idOf('Lithium'));
    },
  },
  {
    name: 'Volcano',
    build() {
      engine.paint_rect(0, 188, WIDTH - 1, HEIGHT - 1, idOf('Stone'));
      engine.paint_line(30, 188, 160, 118, 7, idOf('Stone'));
      engine.paint_line(290, 188, 160, 118, 7, idOf('Stone'));
      engine.paint_rect(120, 150, 200, 188, idOf('Lava'));
      engine.paint_rect(0, 40, WIDTH - 1, 46, idOf('Water'));
      engine.paint_rect(120, 60, 200, 72, idOf('Water'));
    },
  },
  {
    name: 'Density column',
    build() {
      engine.paint_rect(20, 180, 300, 197, idOf('Stone'));
      engine.paint_rect(40, 58, 280, 96, idOf('Oil'));
      engine.paint_rect(40, 24, 280, 56, idOf('Water'));
      engine.paint_rect(40, 2, 280, 22, idOf('Sand'));
      engine.paint(50, 110, 2, idOf('Mercury'));
      engine.paint(270, 110, 2, idOf('Mercury'));
    },
  },
  {
    name: 'Vine',
    build() {
      engine.paint_rect(0, 186, WIDTH - 1, HEIGHT - 1, idOf('Sand'));
      engine.paint_rect(0, 178, WIDTH - 1, 186, idOf('Water'));
      for (let x = 24; x < WIDTH - 10; x += 36) {
        engine.paint(x, 176, 1, idOf('Plant'));
      }
    },
  },
  {
    name: 'Demolition',
    build() {
      engine.paint_rect(80, 186, 240, 196, idOf('Stone'));
      engine.paint_rect(90, 120, 230, 186, idOf('Wood'));
      engine.paint_rect(120, 140, 200, 170, idOf('Tnt'));
      engine.paint_rect(130, 150, 190, 160, idOf('Gunpowder'));
      engine.paint_rect(140, 96, 180, 112, idOf('Hydrogen'));
      engine.paint(160, 92, 2, idOf('Fire'));
    },
  },
  {
    name: 'Carbide cannon',
    build() {
      engine.paint_rect(0, 190, WIDTH - 1, HEIGHT - 1, idOf('Stone'));
      engine.paint_rect(60, 140, 260, 190, idOf('Water'));
      engine.paint_rect(120, 120, 200, 138, idOf('Carbide'));
      engine.paint_rect(0, 30, WIDTH - 1, 34, idOf('Stone'));
    },
  },
];

const presetsEl = document.getElementById('presets');
for (const preset of presets) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = preset.name;
  button.addEventListener('click', () => {
    engine.clear(); // snapshots first, so a preset is undoable
    preset.build();
    setRunning(true);
  });
  presetsEl.append(button);
}

// ---------------------------------------------------------------------------
// go
// ---------------------------------------------------------------------------
// Exposed for poking at the simulation from the browser console, and used by
// tests/dom/main.test.mjs:
//   alkalab.engine.paint(20, 20, 4, alkalab.idOf('Sodium'))
globalThis.alkalab = { engine, wasm, catalog, idOf };

selectElement(selected);
brushOutput.value = String(brushRadius);
setRunning(true);
requestAnimationFrame(frame);


