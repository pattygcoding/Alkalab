# World construction and the per-tick simulation loop.
import world_types, physics, reaction, render, grid, rng, celltype, element
proc newWorld*(width, height: int32, seed: uint32): World =
  ## Allocates the world. The wasm build uses the same `new` as native builds;
  ## `mm:arc` + `-d:useMalloc` gives the standalone target a working allocator.
  new(result)
  result.grid = newGrid(width, height)
  result.rng = rngNew(seed)
  result.frame = 0'u32
  result.pixels = newSeq[uint8](result.grid.gridLen * 4)
  result.background = buildBackground(result.grid)
  result.log = ReactionLog()
  result.undoCount = 0
  for i in 0..<UNDO_LIMIT:
    result.undo[i] = newSeq[Cell](result.grid.gridLen)
  renderWorld(result.pixels, result.background, result.grid)
proc advance(w: World)

proc step*(w: World) =
  w.advance()
  w.renderWorld()

proc stepN*(w: World, ticks: uint32) =
  var i = 0'u32
  while i < ticks:
    w.advance()
    inc i
  w.renderWorld()

proc tickCell(w: World, x, y: int32)

proc advance(w: World) =
  w.frame = w.frame + 1'u32
  if w.frame == 0'u32:
    # The counter wrapped: old stamps must not alias a fresh frame.
    w.frame = 1'u32
    w.grid.clearStamps()
  w.log.beginFrame()
  let leftToRight = (w.frame mod 2) == 0
  let width = w.grid.width
  let height = w.grid.height
  var y = height - 1
  while y >= 0:
    if leftToRight:
      var x = 0'i32
      while x < width:
        w.tickCell(x, y)
        inc x
    else:
      var x = width - 1
      while x >= 0:
        w.tickCell(x, y)
        dec x
    dec y
  w.log.endFrame(w.frame)

proc tickCell(w: World, x, y: int32) =
  ## One cell: chemistry, lifetime, heat, then movement.
  let frame = w.frame
  if w.grid.isMarked(x, y, frame):
    return
  var c = w.grid.cell(x, y)
  if c.isEmpty or c.element == Element.Wall:
    return

  # 1. Chemistry. A reaction may replace this cell entirely.
  if w.react(x, y, c.element):
    return

  # 2. Lifetime. Done before movement so the decremented life travels along.
  c = w.grid.cell(x, y)
  if c.life > 0'u16:
    if c.tickLifetime():
      w.grid.set(x, y, c)
      return
    w.grid.set(x, y, c)

  # 3. Heat sources (fire, lava) ignite whatever fuel touches them.
  if c.element.props.heat > 0'u8:
    w.igniteNeighbours(x, y, c.element)

  # 4. Movement.
  case c.element.category
  of Category.Powder: w.stepPowder(x, y)
  of Category.Liquid: w.stepLiquid(x, y)
  of Category.Gas: w.stepGas(x, y)
  of Category.Fire: w.stepFire(x, y)
  of Category.Solid: w.stepSolid(x, y)
  of Category.Empty: discard
