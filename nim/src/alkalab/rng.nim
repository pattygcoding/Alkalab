# Deterministic xorshift32 PRNG used by the whole simulation.
# -----------------------------------------------------------------------------
# rng - deterministic xorshift32
# -----------------------------------------------------------------------------

type
  Rng* = object
    state*: uint32

func rngNew*(seed: uint32): Rng {.inline.} =
  Rng(state: if seed == 0'u32: 0x9E37_79B9'u32 else: seed)

func nextU32*(r: var Rng): uint32 {.inline.} =
  var x = r.state
  x = x xor (x shl 13)
  x = x xor (x shr 17)
  x = x xor (x shl 5)
  r.state = x
  x

func below*(r: var Rng, n: uint32): uint32 {.inline.} =
  if n == 0'u32: 0'u32 else: r.nextU32() mod n

func chance*(r: var Rng, p: uint8): bool {.inline.} =
  if p == 255'u8: true
  elif p == 0'u8: false
  else: (r.nextU32() and 0xFF'u32) < uint32(p)

func flip*(r: var Rng): bool {.inline.} =
  (r.nextU32() and 1'u32) == 1'u32

func rangeI32*(r: var Rng, lo, hi: int32): int32 {.inline.} =
  if hi <= lo:
    return lo
  lo + int32(r.below(uint32(hi - lo + 1)))

