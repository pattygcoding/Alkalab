# JavaScript facing wasm surface.
import world, world_types, element, json
when defined(wasm):
  {.compile: "../wasm_libc.c".}

  # The browser creates one engine per page, so a single global instance keeps
  # the wasm surface free of pointer ownership games (and of accidental deep
  # copies of the 64 KB JSON scratch buffer).
  var gWorld: World

  proc engineNew*(width, height: int32, seed: uint32): pointer {.exportc: "engine_new", cdecl.} =
    gWorld = newWorld(width, height, seed)
    cast[pointer](gWorld)

  proc engineFree*(e: pointer) {.exportc: "engine_free", cdecl.} =
    discard

  proc engineWidth*(e: pointer): int32 {.exportc: "engine_width", cdecl.} = gWorld.width
  proc engineHeight*(e: pointer): int32 {.exportc: "engine_height", cdecl.} = gWorld.height
  proc engineFrame*(e: pointer): uint32 {.exportc: "engine_frame", cdecl.} = gWorld.frame

  proc enginePixelPtr*(e: pointer): ptr uint8 {.exportc: "engine_pixel_ptr", cdecl.} =
    gWorld.pixelPtr

  proc enginePixelLen*(e: pointer): int32 {.exportc: "engine_pixel_len", cdecl.} =
    int32(gWorld.pixelLen)

  proc engineParticleCount*(e: pointer): uint32 {.exportc: "engine_particle_count", cdecl.} =
    gWorld.particleCount

  proc engineCanUndo*(e: pointer): bool {.exportc: "engine_can_undo", cdecl.} =
    gWorld.canUndo

  proc engineHash*(e: pointer): uint32 {.exportc: "engine_hash", cdecl.} =
    gWorld.hash

  proc engineStep*(e: pointer) {.exportc: "engine_step", cdecl.} =
    gWorld.step()

  proc engineStepN*(e: pointer, ticks: uint32) {.exportc: "engine_step_n", cdecl.} =
    gWorld.stepN(ticks)

  proc enginePaint*(e: pointer, x, y, radius: int32, element: uint8): uint32 {.exportc: "engine_paint", cdecl.} =
    gWorld.paintCircle(x, y, radius, fromId(element))

  proc enginePaintLine*(e: pointer, x0, y0, x1, y1, radius: int32,
                        element: uint8): uint32 {.exportc: "engine_paint_line", cdecl.} =
    gWorld.paintLine(x0, y0, x1, y1, radius, fromId(element))

  proc engineDetonate*(e: pointer, x, y, radius: int32) {.exportc: "engine_detonate", cdecl.} =
    gWorld.explode(x, y, radius, Element.Smoke)
    gWorld.renderWorld()

  proc enginePaintRect*(e: pointer, x0, y0, x1, y1: int32,
                        element: uint8): uint32 {.exportc: "engine_paint_rect", cdecl.} =
    gWorld.paintRect(x0, y0, x1, y1, fromId(element))

  proc engineClear*(e: pointer) {.exportc: "engine_clear", cdecl.} =
    gWorld.clear()

  proc engineSnapshot*(e: pointer) {.exportc: "engine_snapshot", cdecl.} =
    gWorld.snapshot()

  proc engineUndo*(e: pointer): bool {.exportc: "engine_undo", cdecl.} =
    gWorld.undo()

  proc engineElementAt*(e: pointer, x, y: int32): uint8 {.exportc: "engine_element_at", cdecl.} =
    gWorld.element(x, y).id

  proc engineCountOf*(e: pointer, element: uint8): uint32 {.exportc: "engine_count_of", cdecl.} =
    gWorld.count(fromId(element))

  proc engineReactionsPtr*(e: pointer): pointer {.exportc: "engine_reactions_ptr", cdecl.} =
    gWorld.reactionsPtr()

  proc engineReactionsLen*(e: pointer): int32 {.exportc: "engine_reactions_len", cdecl.} =
    int32(gWorld.reactionsLen)

  proc engineCatalogPtr*(e: pointer): pointer {.exportc: "engine_catalog_ptr", cdecl.} =
    gWorld.catalogPtr()

  proc engineCatalogLen*(e: pointer): int32 {.exportc: "engine_catalog_len", cdecl.} =
    int32(gWorld.catalogLen)

