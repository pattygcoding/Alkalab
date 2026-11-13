# World state: reaction log, type and the low-level tools.
import celltype, element, grid, rng, render
from std/math import sqrt
# -----------------------------------------------------------------------------
# world - state buffer, tick loop, reaction log and painting tools
# -----------------------------------------------------------------------------

const
  DEFAULT_WIDTH* = 320'i32
  DEFAULT_HEIGHT* = 200'i32
  UNDO_LIMIT* = 16
  TERMINAL_FALL* = 3'i32
  TERMINAL_RISE* = 4'i32
  MAX_MOMENTUM* = 12'i8
  LOG_CAPACITY* = 8
  JSON_BUFFER_SIZE* = 65536

type
  LogEntry* = object
    equation*: cstring
    count*: uint32
    frame*: uint32

  FrameEntry = object
    equation: cstring
    count: uint32

  ReactionLog* = object
    thisFrame*: array[LOG_CAPACITY, FrameEntry]
    thisFrameCount*: int
    recent*: array[LOG_CAPACITY, LogEntry]
    recentCount*: int

  WorldObj = object
    grid*: Grid
    rng*: Rng
    frame*: uint32
    pixels*: seq[uint8]
    background*: seq[uint8]
    log*: ReactionLog
    undo*: array[UNDO_LIMIT, seq[Cell]]
    undoCount*: int
    jsonBuf*: array[JSON_BUFFER_SIZE, char]
    jsonLen*: int

  World* = ref WorldObj

func cstrEq(a, b: cstring): bool =
  var i = 0
  while a[i] != '\0' and b[i] != '\0':
    if a[i] != b[i]:
      return false
    inc i
  a[i] == '\0' and b[i] == '\0'

proc beginFrame*(l: var ReactionLog) {.inline.} =
  l.thisFrameCount = 0

proc record*(l: var ReactionLog, equation: cstring) =
  for i in 0..<l.thisFrameCount:
    if cstrEq(l.thisFrame[i].equation, equation):
      inc l.thisFrame[i].count
      return
  if l.thisFrameCount < LOG_CAPACITY:
    l.thisFrame[l.thisFrameCount] = FrameEntry(equation: equation, count: 1)
    inc l.thisFrameCount

proc endFrame*(l: var ReactionLog, frame: uint32) =
  for i in 0..<l.thisFrameCount:
    let equation = l.thisFrame[i].equation
    let count = l.thisFrame[i].count
    if l.recentCount > 0 and cstrEq(l.recent[0].equation, equation):
      l.recent[0].count = l.recent[0].count + count
      l.recent[0].frame = frame
      continue
    let last = if l.recentCount < LOG_CAPACITY: l.recentCount else: LOG_CAPACITY - 1
    var j = last
    while j > 0:
      l.recent[j] = l.recent[j - 1]
      dec j
    l.recent[0] = LogEntry(equation: equation, count: count, frame: frame)
    if l.recentCount < LOG_CAPACITY:
      inc l.recentCount

# Forward declarations used by the physics and chemistry passes. The bodies
# live further down in the World section.
proc moveCell*(world: World, src, dst: (int32, int32))
proc moveWithMomentum*(world: World, src, dst: (int32, int32))
proc stopMomentum*(world: World, x, y: int32)
proc logReaction*(world: World, equation: cstring)
proc explode*(world: World, cx, cy: int32, radius: int32, product: Element)

func width*(w: World): int32 {.inline.} = w.grid.width
func height*(w: World): int32 {.inline.} = w.grid.height
func len*(w: World): int {.inline.} = w.grid.gridLen
func isEmpty*(w: World): bool {.inline.} = w.grid.gridIsEmpty
func frame*(w: World): uint32 {.inline.} = w.frame
func grid*(w: World): Grid {.inline.} = w.grid
template gridMut*(w: World): untyped = w.grid
func element*(w: World, x, y: int32): Element {.inline.} = w.grid.element(x, y)
func pixels*(w: World): seq[uint8] {.inline.} = w.pixels
func pixelPtr*(w: World): ptr uint8 {.inline.} = addr w.pixels[0]
func pixelLen*(w: World): int {.inline.} = w.pixels.len
func hash*(w: World): uint32 {.inline.} = w.grid.hash
func particleCount*(w: World): uint32 {.inline.} =
  uint32(w.grid.gridLen) - w.grid.count(Element.Empty)
func count*(w: World, element: Element): uint32 {.inline.} = w.grid.count(element)

proc renderWorld*(w: World) =
  renderWorld(w.pixels, w.background, w.grid)
# --- undo --------------------------------------------------------------------

proc copySnapshot(g: Grid, buf: var seq[Cell]) =
  var i = 0
  for y in 0..<g.height:
    for x in 0..<g.width:
      if i < buf.len:
        buf[i] = g.cell(x, y)
      inc i

proc snapshot*(w: World) =
  ## Remembers the current state so a whole brush stroke can be undone.
  if w.undoCount < UNDO_LIMIT:
    copySnapshot(w.grid, w.undo[w.undoCount])
    inc w.undoCount
  else:
    for i in 0..<UNDO_LIMIT - 1:
      w.undo[i] = w.undo[i + 1]
    copySnapshot(w.grid, w.undo[UNDO_LIMIT - 1])

func canUndo*(w: World): bool {.inline.} = w.undoCount > 0

proc undo*(w: World): bool =
  if w.undoCount == 0:
    return false
  dec w.undoCount
  w.grid.restore(w.undo[w.undoCount])
  w.grid.clearStamps()
  w.renderWorld()
  true
# --- movement plumbing and tools --------------------------------------------

proc moveCell*(world: World, src, dst: (int32, int32)) =
  ## Moves a cell and stamps both ends with the current frame, so nothing can
  ## be simulated twice in one tick.
  world.grid.swap(src[0], src[1], dst[0], dst[1])
  world.grid.mark(src[0], src[1], world.frame)
  world.grid.mark(dst[0], dst[1], world.frame)

proc moveWithMomentum*(world: World, src, dst: (int32, int32)) =
  var c = world.grid.cell(src[0], src[1])
  if c.vel < MAX_MOMENTUM:
    inc c.vel
  world.grid.set(src[0], src[1], c)
  world.moveCell(src, dst)

proc stopMomentum*(world: World, x, y: int32) =
  var c = world.grid.cell(x, y)
  if c.vel != 0'i8:
    c.vel = 0'i8
    world.grid.set(x, y, c)

proc logReaction*(world: World, equation: cstring) =
  world.log.record(equation)

proc explode*(world: World, cx, cy: int32, radius: int32, product: Element) =
  ## Blows a hole in the world. Wall is immune.
  let r = clampI32(radius, 1, 32)
  let r2 = r * r
  let inner = r div 2
  var dy = -r
  while dy <= r:
    var dx = -r
    while dx <= r:
      let d2 = dx * dx + dy * dy
      if d2 <= r2:
        let x = cx + dx
        let y = cy + dy
        if world.grid.inBounds(x, y) and world.grid.element(x, y) != Element.Wall:
          let falloff = float32(sqrt(float64(d2))) / float32(r)
          let destroy = uint8(255.0 - falloff * 195.0)
          if world.rng.chance(destroy):
            if d2 <= inner * inner:
              world.grid.setTransient(x, y, Element.Fire, 36, Element.Smoke)
            elif world.rng.chance(150):
              world.grid.setTransient(x, y, Element.Fire, 20, Element.Smoke)
            elif product != Element.Empty and world.rng.chance(110):
              let p = product.props
              world.grid.setTransient(x, y, product, p.life, p.residue)
            else:
              world.grid.set(x, y, cellEmpty())
            # Blast material sits out the rest of the tick.
            world.grid.mark(x, y, world.frame)
      inc dx
    inc dy

proc paintCircleRaw(w: World, cx, cy: int32, radius: int32,
                    element: Element): uint32 =
  let r = clampI32(radius, 0, 64)
  let r2 = r * r
  let cell = newCell(element)
  var painted = 0'u32
  var dy = -r
  while dy <= r:
    var dx = -r
    while dx <= r:
      if dx * dx + dy * dy <= r2:
        let x = cx + dx
        let y = cy + dy
        if w.grid.inBounds(x, y):
          w.grid.set(x, y, cell)
          inc painted
      inc dx
    inc dy
  painted

proc paintCircle*(w: World, cx, cy: int32, radius: int32,
                  element: Element): uint32 =
  result = w.paintCircleRaw(cx, cy, radius, element)
  w.renderWorld()

proc paintLine*(w: World, x0, y0, x1, y1: int32, radius: int32,
                element: Element): uint32 =
  ## Interpolation-free line of circles, so fast drags do not leave gaps.
  let dx = x1 - x0
  let dy = y1 - y0
  let steps = max(max(abs(dx), abs(dy)), 1'i32)
  var painted = 0'u32
  var i = 0'i32
  while i <= steps:
    let x = x0 + dx * i div steps
    let y = y0 + dy * i div steps
    painted += w.paintCircleRaw(x, y, radius, element)
    inc i
  w.renderWorld()
  painted

proc paintRect*(w: World, x0, y0, x1, y1: int32, element: Element): uint32 =
  ## Paints a rectangle (inclusive corners).
  let ax = min(x0, x1)
  let bx = max(x0, x1)
  let ay = min(y0, y1)
  let by = max(y0, y1)
  let cell = newCell(element)
  var painted = 0'u32
  var y = ay
  while y <= by:
    var x = ax
    while x <= bx:
      if w.grid.inBounds(x, y):
        w.grid.set(x, y, cell)
        inc painted
      inc x
    inc y
  w.renderWorld()
  painted

proc clear*(w: World) =
  ## Empties the world (undoable).
  w.snapshot()
  w.grid.clear(Element.Empty)
  w.grid.clearStamps()
  w.renderWorld()

