// Copies the files a portfolio (or any other static host) needs into dist/.
//
//   npm run build && npm run bundle
//
// Only the runtime files are copied - the TypeScript declarations and the Rust
// source stay behind. The layout is preserved, because both links are relative:
//
//   index.html -> ./style.css
//              -> ./main.js -> ./pkg/alkalab.js -> ./alkalab.wasm
//
// Serve it with `node scripts/serve.mjs dist` and the whole lab runs.

import { copyFile, mkdir, readdir, rm, stat } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';

const OUT = 'dist';

/** Everything the page actually needs at runtime, in load order. */
const FILES = [
  'web/index.html',
  'web/style.css',
  'web/main.js',
  'web/pkg/alkalab.js', // the wasm-bindgen glue: required, the binary alone is not loadable
  'web/pkg/alkalab.wasm',
];

const missing = FILES.filter((file) => !existsSync(file));
if (missing.length > 0) {
  console.error(`missing: ${missing.join(', ')}`);
  console.error('run `npm run build` first.');
  process.exit(1);
}

await rm(OUT, { recursive: true, force: true });

let total = 0;
const rows = [];
for (const file of FILES) {
  const relative = file.replace(/^web[\\/]/, '');
  const target = join(OUT, relative);
  await mkdir(dirname(target), { recursive: true });
  await copyFile(file, target);
  const { size } = await stat(target);
  total += size;
  rows.push([relative, size]);
}

console.log(`${OUT}/`);
for (const [name, size] of rows) {
  console.log(`  ${name.padEnd(22)} ${size.toLocaleString().padStart(9)} bytes`);
}
console.log(`  ${''.padEnd(22)} ${'-'.repeat(9)}`);
console.log(`  ${'total'.padEnd(22)} ${total.toLocaleString().padStart(9)} bytes`);

// Sanity check: the glue must point at the binary that was copied next to it.
const { readFile } = await import('node:fs/promises');
const glue = await readFile(join(OUT, 'pkg/alkalab.js'), 'utf8');
if (!glue.includes("new URL('alkalab.wasm', import.meta.url)")) {
  throw new Error('the glue does not reference alkalab.wasm - rebuild before bundling');
}

const listing = await readdir(OUT);
console.log(`\n${listing.length} entries in ${OUT}/ - serve it with:`);
console.log(`  node scripts/serve.mjs ${OUT}`);
