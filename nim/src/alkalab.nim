# =============================================================================
# Alkalab - a powder-sand / cellular-automata chemistry lab.
#
# The engine is split into modules under `alkalab/`. This root module only
# re-exports the public surface, so `import alkalab` keeps working for the
# tests, the wasm build and any native consumer.
#
# License: MIT
# =============================================================================

import alkalab/[
  color, rng, element, celltype, grid, neighbours, render, world_types,
  json, physics, reaction, world
]

export
  color, rng, element, celltype, grid, neighbours, render, world_types,
  json, physics, reaction, world

when defined(wasm):
  import alkalab/wasm_api
  export wasm_api