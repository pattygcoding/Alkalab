# Colour helpers: packed little-endian RGBA, blending and positional noise.
# -----------------------------------------------------------------------------
# color - packed little-endian RGBA helpers
# -----------------------------------------------------------------------------
# Colours are packed as 0xAABBGGRR so that a uint32 can be copied straight into
# the shared byte buffer and read as [R, G, B, A] by ImageData.

func rgba*(r, g, b, a: uint8): uint32 {.inline.} =
  uint32(r) or (uint32(g) shl 8) or (uint32(b) shl 16) or (uint32(a) shl 24)

func rgb*(r, g, b: uint8): uint32 {.inline.} =
  rgba(r, g, b, 255'u8)

func red*(color: uint32): uint8 {.inline.} = uint8(color and 0xFF'u32)
func green*(color: uint32): uint8 {.inline.} = uint8((color shr 8) and 0xFF'u32)
func blue*(color: uint32): uint8 {.inline.} = uint8((color shr 16) and 0xFF'u32)
func alpha*(color: uint32): uint8 {.inline.} = uint8((color shr 24) and 0xFF'u32)

const HEX_DIGITS: array[16, char] =
  ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
   'a', 'b', 'c', 'd', 'e', 'f']

func toHex*(color: uint32): array[7, char] {.inline.} =
  ## "#rrggbb" - used by the HTML palette.
  result[0] = '#'
  result[1] = HEX_DIGITS[int(red(color) shr 4)]
  result[2] = HEX_DIGITS[int(red(color) and 0x0F'u8)]
  result[3] = HEX_DIGITS[int(green(color) shr 4)]
  result[4] = HEX_DIGITS[int(green(color) and 0x0F'u8)]
  result[5] = HEX_DIGITS[int(blue(color) shr 4)]
  result[6] = HEX_DIGITS[int(blue(color) and 0x0F'u8)]

proc toHexInto*(color: uint32, buf: var array[7, char]) =
  ## Same as toHex but writes into a caller supplied buffer, which avoids
  ## returning a fixed array by value across the wasm ABI.
  buf[0] = '#'
  buf[1] = HEX_DIGITS[int(red(color) shr 4)]
  buf[2] = HEX_DIGITS[int(red(color) and 0x0F'u8)]
  buf[3] = HEX_DIGITS[int(green(color) shr 4)]
  buf[4] = HEX_DIGITS[int(green(color) and 0x0F'u8)]
  buf[5] = HEX_DIGITS[int(blue(color) shr 4)]
  buf[6] = HEX_DIGITS[int(blue(color) and 0x0F'u8)]

func clampChannel(c: int16): uint8 {.inline.} =
  if c < 0: 0'u8 elif c > 255: 255'u8 else: uint8(c)

func shade*(color: uint32, delta: int16): uint32 {.inline.} =
  if delta == 0:
    return color
  rgba(clampChannel(int16(red(color)) + delta),
       clampChannel(int16(green(color)) + delta),
       clampChannel(int16(blue(color)) + delta),
       alpha(color))

func mix*(a, b: uint32, t: uint8): uint32 {.inline.} =
  func lerp(x, y: uint8, t: uint8): uint8 =
    let tt = uint32(t)
    uint8((uint32(x) * (255'u32 - tt) + uint32(y) * tt) div 255'u32)
  rgba(lerp(red(a), red(b), t),
       lerp(green(a), green(b), t),
       lerp(blue(a), blue(b), t),
       lerp(alpha(a), alpha(b), t))

func blend*(dst, src: uint32): uint32 {.inline.} =
  ## Source-over blend of `src` (possibly translucent) on top of `dst`.
  let sa = alpha(src)
  if sa == 255'u8:
    return src
  if sa == 0'u8:
    return dst
  func over(d, s: uint8, a: uint8): uint8 =
    let aa = uint32(a)
    uint8((uint32(s) * aa + uint32(d) * (255'u32 - aa)) div 255'u32)
  rgba(over(red(dst), red(src), sa),
       over(green(dst), green(src), sa),
       over(blue(dst), blue(src), sa),
       255'u8)

func noise2d*(x, y: int32, salt: uint32): uint8 {.inline.} =
  ## Cheap deterministic positional noise in 0..=255.
  var h = (uint32(x) * 0x27D4_EB2D'u32) xor (uint32(y) * 0x1656_67B1'u32) xor salt
  h = h xor (h shr 15)
  h = h * 0x2545_F491'u32
  h = h xor (h shr 13)
  uint8(h and 0xFF'u32)

