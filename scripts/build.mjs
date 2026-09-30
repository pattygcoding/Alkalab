// Builds the WebAssembly package into web/pkg.
//
//   node scripts/build.mjs [--no-opt]
//
// wasm-pack compiles the crate and generates the ES module glue; wasm-bindgen
// always names the binary `<out-name>_bg.wasm`, so this script renames it to
// `alkalab.wasm` and points the generated glue at the new name (there is exactly
// one reference, and the build fails loudly if that changes).
//
// Cross platform on purpose: it is what `npm run build` and CI run.

import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { readFile, readdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const OUT_DIR = 'web/pkg';
/** What wasm-bindgen calls the module. */
const GENERATED_BINARY = 'alkalab_bg.wasm';
/** What we want it called. */
const BINARY = 'alkalab.wasm';
const GLUE = 'alkalab.js';

const skipOpt = process.argv.includes('--no-opt');
const isWindows = process.platform === 'win32';

function run(command, args) {
  const result = spawnSync(command, args, { stdio: 'inherit' });
  if (result.error) throw result.error;
  return result.status ?? 1;
}

/** `wasm-pack` on PATH, otherwise a release unpacked into tools/. */
function findWasmPack() {
  const probe = spawnSync('wasm-pack', ['--version'], { stdio: 'ignore' });
  if (!probe.error && probe.status === 0) return 'wasm-pack';

  const exe = isWindows ? 'wasm-pack.exe' : 'wasm-pack';
  if (existsSync('tools')) {
    for (const entry of readdirSyncSafe('tools')) {
      const candidate = join('tools', entry, exe);
      if (existsSync(candidate)) return candidate;
      const nested = join('tools', entry, 'bin', exe);
      if (existsSync(nested)) return nested;
    }
  }
  throw new Error(
    [
      'wasm-pack was not found.',
      'Install it with `cargo install wasm-pack`, or drop a release into tools/:',
      '  https://rustwasm.github.io/wasm-pack/installer/',
      'Windows example:',
      "  Invoke-WebRequest 'https://github.com/rustwasm/wasm-pack/releases/download/v0.15.0/wasm-pack-v0.15.0-x86_64-pc-windows-msvc.tar.gz' -OutFile tools/wasm-pack.tar.gz",
      '  tar -xzf tools/wasm-pack.tar.gz -C tools',
    ].join('\n'),
  );
}

/** Directory listing that never throws (tools/ may not exist). */
function readdirSyncSafe(dir) {
  try {
    return readdirSync(dir);
  } catch {
    return [];
  }
}

const wasmPack = findWasmPack();
console.log(`using ${wasmPack}`);

// Always start clean, so a stale binary can never survive a failed build.
await rm(OUT_DIR, { recursive: true, force: true });

console.log('checking the wasm32-unknown-unknown target');
run('rustup', ['target', 'add', 'wasm32-unknown-unknown']);

const buildArgs = ['build', '--target', 'web', '--out-dir', OUT_DIR, '--no-pack'];
if (skipOpt) buildArgs.push('--no-opt');

console.log(`building -> ${OUT_DIR}`);
if (run(wasmPack, buildArgs) !== 0) {
  if (skipOpt) throw new Error('wasm-pack build failed');
  console.warn('wasm-pack failed (wasm-opt usually), retrying without optimisation');
  if (run(wasmPack, [...buildArgs, '--no-opt']) !== 0) {
    throw new Error('wasm-pack build failed');
  }
}

// --- name the binary ---------------------------------------------------------
const generatedPath = join(OUT_DIR, GENERATED_BINARY);
const binaryPath = join(OUT_DIR, BINARY);
if (!existsSync(generatedPath)) {
  throw new Error(`${generatedPath} was not produced - did wasm-bindgen change its output naming?`);
}
await rename(generatedPath, binaryPath);

// The generated glue hardcodes the module URL and the raw declaration file is
// named after the module, so keep both in sync with the rename.
const gluePath = join(OUT_DIR, GLUE);
const glue = await readFile(gluePath, 'utf8');
if (!glue.includes(`'${GENERATED_BINARY}'`)) {
  throw new Error(`no reference to '${GENERATED_BINARY}' in ${GLUE} - nothing to patch`);
}
await writeFile(gluePath, glue.replaceAll(`'${GENERATED_BINARY}'`, `'${BINARY}'`));

const declarations = join(OUT_DIR, `${GENERATED_BINARY}.d.ts`);
if (existsSync(declarations)) {
  await rename(declarations, join(OUT_DIR, `${BINARY}.d.ts`));
}

// --- sanity check and report -------------------------------------------------
const { size } = await stat(binaryPath);
const magic = (await readFile(binaryPath)).subarray(0, 4);
if (magic[0] !== 0x00 || magic[1] !== 0x61 || magic[2] !== 0x73 || magic[3] !== 0x6d) {
  throw new Error(`${binaryPath} is not a WebAssembly module`);
}

const files = (await readdir(OUT_DIR)).filter((name) => name !== '.gitignore');
console.log(`\n${OUT_DIR}/`);
for (const name of files) {
  const info = await stat(join(OUT_DIR, name));
  console.log(`  ${name.padEnd(20)} ${info.size.toLocaleString().padStart(10)} bytes`);
}
console.log(`\nweb/pkg/${BINARY} ready: ${size.toLocaleString()} bytes of wasm`);
