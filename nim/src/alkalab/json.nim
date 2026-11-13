# JSON builders for the element catalogue and the reaction log.
import world_types, element, color
# --- JSON builders -----------------------------------------------------------
# The buffers are plain char arrays so the wasm build never allocates when the
# front end polls reactions each frame.

proc jsonReset(w: World) = w.jsonLen = 0

proc jsonPut(w: World, c: char) =
  if w.jsonLen < JSON_BUFFER_SIZE:
    w.jsonBuf[w.jsonLen] = c
    inc w.jsonLen

proc jsonPut(w: World, s: cstring) =
  var i = 0
  while s[i] != '\0':
    w.jsonPut(s[i])
    inc i

proc jsonPutU32(w: World, v: uint32) =
  if v == 0'u32:
    w.jsonPut('0')
    return
  var digits: array[10, char]
  var n = 0
  var x = v
  while x > 0'u32:
    digits[n] = char(ord('0') + int(x mod 10'u32))
    inc n
    x = x div 10'u32
  while n > 0:
    dec n
    w.jsonPut(digits[n])

proc buildCatalogJson(w: World) =
  w.jsonReset()
  w.jsonPut('[')
  for i in 0..<ELEMENT_COUNT:
    if i > 0:
      w.jsonPut(',')
    let e = ELEMENT_ALL[i]
    let pi = ord(e)
    w.jsonPut("{\"id\":")
    w.jsonPutU32(uint32(e.id))
    w.jsonPut(",\"name\":\"")
    w.jsonPut(PROP_TABLE[pi].name)
    w.jsonPut("\",\"formula\":\"")
    w.jsonPut(PROP_TABLE[pi].formula)
    w.jsonPut("\",\"category\":\"")
    w.jsonPut(categoryName(PROP_TABLE[pi].category))
    w.jsonPut("\",\"color\":\"")
    var hex: array[7, char]
    toHexInto(PROP_TABLE[pi].color, hex)
    for c in hex:
      w.jsonPut(c)
    w.jsonPut("\",\"density\":")
    w.jsonPutU32(uint32(PROP_TABLE[pi].density))
    w.jsonPut(",\"desc\":\"")
    w.jsonPut(PROP_TABLE[pi].desc)
    w.jsonPut("\"}")
  w.jsonPut(']')

proc buildReactionsJson(w: World) =
  w.jsonReset()
  w.jsonPut('[')
  for i in 0..<w.log.recentCount:
    if i > 0:
      w.jsonPut(',')
    let entry = w.log.recent[i]
    w.jsonPut("{\"equation\":\"")
    w.jsonPut(entry.equation)
    w.jsonPut("\",\"count\":")
    w.jsonPutU32(entry.count)
    w.jsonPut(",\"frame\":")
    w.jsonPutU32(entry.frame)
    w.jsonPut('}')
  w.jsonPut(']')

proc catalogPtr*(w: World): pointer =
  w.buildCatalogJson()
  addr w.jsonBuf[0]

func catalogLen*(w: World): int {.inline.} = w.jsonLen

proc reactionsPtr*(w: World): pointer =
  w.buildReactionsJson()
  addr w.jsonBuf[0]

func reactionsLen*(w: World): int {.inline.} = w.jsonLen
