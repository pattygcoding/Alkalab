# Flat, bordered cell buffer plus a per-frame update stamp.
import celltype, element
# -----------------------------------------------------------------------------
# grid - a flat bordered cell buffer plus a per-frame stamp
# -----------------------------------------------------------------------------

const
  BORDER_ELEMENT* = Element.Wall
  MIN_SIZE* = 8'i32
  MAX_SIZE* = 2048'i32

func clampI32*(v, lo, hi: int32): int32 {.inline.} =
  if v < lo: lo elif v > hi: hi else: v

type
  Grid* = object
    width*: int32
    height*: int32
    stride*: int32
    cells*: seq[Cell]
    stamp*: seq[uint32]

proc newGrid*(width, height: int32): Grid =
  let w = clampI32(width, MIN_SIZE, MAX_SIZE)
  let h = clampI32(height, MIN_SIZE, MAX_SIZE)
  let stride = w + 2
  let total = int(stride) * int(h + 2)
  var cells = newSeq[Cell](total)
  for i in 0..<total:
    cells[i] = newCell(BORDER_ELEMENT)
  for y in 0..<h:
    for x in 0..<w:
      cells[int((y + 1) * stride + (x + 1))] = cellEmpty()
  Grid(width: w, height: h, stride: stride, cells: cells,
       stamp: newSeq[uint32](total))

func gridWidth*(g: Grid): int32 {.inline.} = g.width
func gridHeight*(g: Grid): int32 {.inline.} = g.height
func gridLen*(g: Grid): int {.inline.} = int(g.width) * int(g.height)
func gridIsEmpty*(g: Grid): bool {.inline.} = g.gridLen == 0

func inBounds*(g: Grid, x, y: int32): bool {.inline.} =
  x >= 0 and y >= 0 and x < g.width and y < g.height

func index*(g: Grid, x, y: int32): int {.inline.} =
  ## Out of range coordinates resolve to the inert border, so neighbour
  ## lookups are always safe and never wrap around.
  let cx = clampI32(x, -1, g.width)
  let cy = clampI32(y, -1, g.height)
  int((cy + 1) * g.stride + (cx + 1))

func cell*(g: Grid, x, y: int32): Cell {.inline.} = g.cells[g.index(x, y)]
func element*(g: Grid, x, y: int32): Element {.inline.} = g.cell(x, y).element

proc set*(g: var Grid, x, y: int32, cell: Cell) {.inline.} =
  g.cells[g.index(x, y)] = cell

proc setElement*(g: var Grid, x, y: int32, element: Element) {.inline.} =
  g.set(x, y, newCell(element))

proc setTransient*(g: var Grid, x, y: int32, element: Element, life: uint16,
                   residue: Element) {.inline.} =
  g.set(x, y, cellTransient(element, life, residue))

proc replace*(g: var Grid, x, y: int32, expected: Element, cell: Cell): bool {.inline.} =
  if g.element(x, y) == expected:
    g.set(x, y, cell)
    true
  else:
    false

proc swap*(g: var Grid, x0, y0, x1, y1: int32) {.inline.} =
  let a = g.index(x0, y0)
  let b = g.index(x1, y1)
  let tmp = g.cells[a]
  g.cells[a] = g.cells[b]
  g.cells[b] = tmp

proc mark*(g: var Grid, x, y: int32, frame: uint32) {.inline.} =
  g.stamp[g.index(x, y)] = frame

func isMarked*(g: Grid, x, y: int32, frame: uint32): bool {.inline.} =
  g.stamp[g.index(x, y)] == frame

proc clearStamps*(g: var Grid) =
  for i in 0..<g.stamp.len:
    g.stamp[i] = 0

proc snapshot*(g: Grid): seq[Cell] =
  result = newSeq[Cell](g.gridLen)
  var i = 0
  for y in 0..<g.height:
    for x in 0..<g.width:
      result[i] = g.cell(x, y)
      inc i

proc restore*(g: var Grid, snapshot: seq[Cell]) =
  var i = 0
  for y in 0..<g.height:
    for x in 0..<g.width:
      if i < snapshot.len:
        g.cells[g.index(x, y)] = snapshot[i]
      inc i

proc clear*(g: var Grid, fill: Element) =
  let cell = newCell(fill)
  for y in 0..<g.height:
    for x in 0..<g.width:
      g.cells[g.index(x, y)] = cell

proc count*(g: Grid, element: Element): uint32 =
  var total = 0'u32
  for y in 0..<g.height:
    for x in 0..<g.width:
      if g.cell(x, y).element == element:
        inc total
  total

func hash*(g: Grid): uint32 =
  ## Order independent FNV style fingerprint of the state buffer.
  var h = 0x811C_9DC5'u32
  for y in 0..<g.height:
    for x in 0..<g.width:
      let c = g.cell(x, y)
      h = (h xor uint32(c.element.id)) * 0x0100_0193'u32
      h = (h xor uint32(c.variant)) * 0x0100_0193'u32
      h = (h xor uint32(c.life and 0xFF'u16)) * 0x0100_0193'u32
      h = (h xor uint32(c.life shr 8)) * 0x0100_0193'u32
      h = (h xor uint32(uint8(c.vel))) * 0x0100_0193'u32
      h = (h xor uint32(c.residue.id)) * 0x0100_0193'u32
  h

