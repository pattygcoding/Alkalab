# Renderer: grid state -> packed RGBA pixel buffer.
import color, celltype, element, grid
# -----------------------------------------------------------------------------
# render - grid state -> shared RGBA pixel buffer
# -----------------------------------------------------------------------------

const
  FLAME_FLICKER* = 14'i16
  AIR_GRAIN* = 5'i16

func readPixel(buffer: seq[uint8], offset: int): uint32 {.inline.} =
  uint32(buffer[offset]) or
    (uint32(buffer[offset + 1]) shl 8) or
    (uint32(buffer[offset + 2]) shl 16) or
    (uint32(buffer[offset + 3]) shl 24)

proc writePixel(buffer: var seq[uint8], offset: int, color: uint32) {.inline.} =
  buffer[offset] = uint8(color and 0xFF'u32)
  buffer[offset + 1] = uint8((color shr 8) and 0xFF'u32)
  buffer[offset + 2] = uint8((color shr 16) and 0xFF'u32)
  buffer[offset + 3] = uint8((color shr 24) and 0xFF'u32)

proc buildBackground*(grid: Grid): seq[uint8] =
  ## Pre-renders the empty world, grain included, so every frame can start
  ## from an opaque image and translucent elements blend over it cleanly.
  result = newSeq[uint8](grid.gridLen * 4)
  let air = Element.Empty.colorOf
  for y in 0..<grid.height:
    for x in 0..<grid.width:
      let grain = int16(noise2d(x, y, 0x51ED_2701'u32)) div 40 - AIR_GRAIN
      let offset = (int(y) * int(grid.width) + int(x)) * 4
      result.writePixel(offset, shade(air, grain))

func flameColor(life: uint16, variant: uint8, x, y: int32): uint32 =
  const
    RED = rgba(178, 38, 14, 235)
    ORANGE = rgba(246, 130, 25, 235)
    YELLOW = rgba(255, 198, 72, 235)
    WHITE = rgba(255, 246, 214, 240)
  let cappedLife = if life > 140'u16: 140'u16 else: life
  let t = uint8(uint32(cappedLife) * 255'u32 div 140'u32)
  var base: uint32
  if t < 85'u8:
    base = mix(RED, ORANGE, uint8(uint32(t) * 3'u32))
  elif t < 170'u8:
    base = mix(ORANGE, YELLOW, uint8((uint32(t) - 85'u32) * 3'u32))
  else:
    base = mix(YELLOW, WHITE, uint8((uint32(t) - 170'u32) * 3'u32))
  if variant == VARIANT_HOT:
    base = mix(base, WHITE, 210'u8)
  let flicker = int16(noise2d(x, y, 0x00F1_A11E'u32)) div 16 - 8
  let clamped = if flicker < -FLAME_FLICKER: -FLAME_FLICKER
                elif flicker > FLAME_FLICKER: FLAME_FLICKER
                else: flicker
  shade(base, clamped)

func elementColor*(element: Element, variant: uint8, life: uint16,
                   x, y: int32): uint32 =
  if element == Element.Fire:
    return flameColor(life, variant, x, y)
  let p = element.props
  if p.speckle == 0:
    return p.color
  let noise = int16(noise2d(x, y, uint32(element.id) * 7919'u32))
  let delta = (noise - 128) * int16(p.speckle) div 160
  shade(p.color, delta)

proc renderWorld*(pixels, background: var seq[uint8], grid: Grid) =
  ## Redraws the shared pixel buffer from the state buffer.
  for i in 0..<background.len:
    pixels[i] = background[i]
  for y in 0..<grid.height:
    for x in 0..<grid.width:
      let c = grid.cell(x, y)
      if c.element == Element.Empty:
        continue
      let color = elementColor(c.element, c.variant, c.life, x, y)
      let offset = (int(y) * int(grid.width) + int(x)) * 4
      if alpha(color) == 255'u8:
        pixels.writePixel(offset, color)
      else:
        let under = readPixel(pixels, offset)
        pixels.writePixel(offset, blend(under, color))

