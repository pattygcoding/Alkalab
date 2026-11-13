# Native test suite for the Nim Alkalab engine.
#
#   nim c -r nim/tests/alkalab_tests.nim
#
# The simulation core is plain Nim, so the whole behaviour can be tested
# without a browser. The wasm build is exercised separately by the smoke test.

import std/[unittest, strutils]
import ../src/alkalab

func jsonString(w: World, catalog: bool): string =
  if catalog:
    discard w.catalogPtr()
  else:
    discard w.reactionsPtr()
  let n = if catalog: w.catalogLen else: w.reactionsLen
  result = newString(n)
  for i in 0..<n:
    result[i] = w.jsonBuf[i]

suite "color":
  test "channels round trip":
    let c = rgba(1, 2, 3, 4)
    check red(c) == 1'u8
    check green(c) == 2'u8
    check blue(c) == 3'u8
    check alpha(c) == 4'u8

  test "hex formatting":
    check toHex(rgb(0, 0, 0)) == ['#', '0', '0', '0', '0', '0', '0']
    check toHex(rgb(255, 128, 16)) == ['#', 'f', 'f', '8', '0', '1', '0']

  test "shade clamps":
    let c = rgba(250, 5, 128, 33)
    check red(shade(c, 20)) == 255'u8
    check green(shade(c, -20)) == 0'u8
    check blue(shade(c, 0)) == 128'u8
    check alpha(shade(c, 20)) == 33'u8

  test "blend respects alpha":
    let bg = rgb(100, 100, 100)
    check blend(bg, rgb(0, 0, 0)) == rgb(0, 0, 0)
    check blend(bg, rgba(0, 0, 0, 0)) == bg
    check alpha(blend(bg, rgba(0, 0, 0, 128))) == 255'u8

suite "rng":
  test "deterministic for the same seed":
    var a = rngNew(1234)
    var b = rngNew(1234)
    for _ in 0..<1000:
      check a.nextU32() == b.nextU32()

  test "different seeds diverge":
    var a = rngNew(1)
    var b = rngNew(2)
    check a.nextU32() != b.nextU32()

  test "zero seed is remapped":
    var r = rngNew(0)
    check r.nextU32() != 0'u32

  test "chance bounds":
    var r = rngNew(42)
    for _ in 0..<1000:
      check not r.chance(0)
      check r.chance(255)

suite "cell":
  test "new uses element defaults":
    let fire = newCell(Element.Fire)
    check fire.life == Element.Fire.props.life
    check fire.residue == Element.Fire.props.residue

  test "lifetime expires into residue":
    var steam = cellTransient(Element.Steam, 3, Element.Water)
    check not steam.tickLifetime()
    check not steam.tickLifetime()
    check steam.tickLifetime()
    check steam.element == Element.Water

  test "immortal cells never expire":
    var sand = newCell(Element.Sand)
    for _ in 0..<1000:
      check not sand.tickLifetime()
    check sand.element == Element.Sand

  test "wet flag round trips":
    var powder = newCell(Element.Gunpowder)
    check not powder.isWet
    powder.variant = VARIANT_WET
    check powder.isWet

suite "element table":
  test "ids match table order":
    check ELEMENT_COUNT == 40
    check ELEMENT_ALL.len == ELEMENT_COUNT
    for i in 0..<ELEMENT_COUNT:
      let e = ELEMENT_ALL[i]
      check e.id == uint8(i)
      check e.name != nil
      check e.formula != nil
      check e.props.name == e.name

  test "lookup helpers":
    check fromId(0) == Element.Empty
    check fromId(uint8(Element.Water.id)) == Element.Water
    check fromId(255) == Element.Empty
    check byName("water") == Element.Water
    check byName("unobtanium") == Element.Empty

  test "categories match behaviour":
    check Element.Sand.category == Category.Powder
    check Element.Water.category == Category.Liquid
    check Element.Steam.category == Category.Gas
    check Element.Fire.category == Category.Fire
    check not Element.Stone.isMobile

  test "new elements are present":
    check byName("thermite") == Element.Thermite
    check byName("methane") == Element.Methane
    check byName("coal") == Element.Coal
    check byName("dry ice") == Element.DryIce
    check byName("liquid nitrogen") == Element.LiquidNitrogen
    check byName("firework") == Element.Firework
    check byName("copper") == Element.Copper
    check byName("gold") == Element.Gold

  test "new elements keep their categories":
    check Element.Thermite.category == Category.Powder
    check Element.Methane.category == Category.Gas
    check Element.Coal.category == Category.Solid
    check Element.DryIce.category == Category.Solid
    check Element.LiquidNitrogen.category == Category.Liquid
    check Element.Firework.category == Category.Powder
    check Element.Copper.category == Category.Solid
    check Element.Gold.category == Category.Solid
    check Element.Methane.isMobile
    check Element.LiquidNitrogen.isMobile
    check not Element.Coal.isMobile

suite "grid":
  test "border is wall and interior is empty":
    let g = newGrid(8, 8)
    check g.element(-1, 0) == Element.Wall
    check g.element(0, -1) == Element.Wall
    check g.element(8, 8) == Element.Wall
    check g.element(0, 0) == Element.Empty

  test "far out of range coordinates are safe":
    let g = newGrid(8, 8)
    check g.element(-10000, -10000) == Element.Wall
    check g.element(10000, 10000) == Element.Wall

  test "set swap and replace":
    var g = newGrid(8, 8)
    g.setElement(2, 2, Element.Sand)
    g.setElement(3, 2, Element.Water)
    check g.element(2, 2) == Element.Sand
    g.swap(2, 2, 3, 2)
    check g.element(2, 2) == Element.Water
    check g.element(3, 2) == Element.Sand
    check not g.replace(2, 2, Element.Sand, cellEmpty())
    check g.replace(3, 2, Element.Sand, cellEmpty())

  test "snapshot and restore":
    var g = newGrid(8, 8)
    g.setElement(1, 1, Element.Sand)
    let snap = g.snapshot()
    g.setElement(1, 1, Element.Water)
    check g.element(1, 1) == Element.Water
    g.restore(snap)
    check g.element(1, 1) == Element.Sand


suite "world":
  test "painting a circle covers the disc":
    var w = newWorld(32, 24, 42)
    let painted = w.paintCircle(16, 12, 3, Element.Sand)
    check painted == 29'u32
    check w.count(Element.Sand) == painted
    check w.element(16, 12) == Element.Sand
    check w.element(16, 15) == Element.Sand
    check w.element(16, 16) == Element.Empty

  test "sand falls and is conserved":
    var w = newWorld(32, 24, 42)
    discard w.paintCircle(16, 2, 2, Element.Sand)
    let before = w.count(Element.Sand)
    w.stepN(80)
    check w.count(Element.Sand) == before
    check w.element(16, 23) == Element.Sand

  test "water spreads and is conserved":
    var w = newWorld(32, 24, 42)
    discard w.paintRect(10, 2, 12, 6, Element.Water)
    let before = w.count(Element.Water)
    w.stepN(80)
    check w.count(Element.Water) == before
    check w.element(11, 23) == Element.Water

  test "gas rises":
    var w = newWorld(32, 24, 42)
    discard w.paintCircle(16, 22, 2, Element.Hydrogen)
    w.stepN(10)
    var high = false
    for y in 0..<20:
      for x in 0..<32:
        if w.element(x.int32, y.int32) == Element.Hydrogen:
          high = true
    check w.count(Element.Hydrogen) > 0
    check high

  test "sodium detonates in water":
    var w = newWorld(32, 24, 2024)
    discard w.paintRect(8, 14, 24, 23, Element.Water)
    w.gridMut.setElement(16, 12, Element.Sodium)
    var saw = false
    for _ in 0..<240:
      w.step()
      if not saw and jsonString(w, false).contains("2 Na + 2 H2O"):
        saw = true
    check w.count(Element.Sodium) == 0
    check saw

  test "carbide fizzes acetylene":
    var w = newWorld(32, 24, 7)
    discard w.paintRect(8, 16, 22, 23, Element.Water)
    w.gridMut.setElement(14, 12, Element.Carbide)
    w.stepN(200)
    let log = jsonString(w, false)
    check log.contains("CaC2")
    check w.count(Element.Carbide) == 0

  test "acid dissolves metal":
    var w = newWorld(32, 24, 5)
    discard w.paintRect(10, 14, 13, 19, Element.Metal)
    discard w.paintRect(10, 9, 13, 12, Element.Acid)
    let before = w.count(Element.Metal)
    w.stepN(400)
    check w.count(Element.Metal) < before

  test "lava fuses sand into glass":
    var w = newWorld(32, 24, 9)
    discard w.paintRect(10, 17, 14, 20, Element.Sand)
    discard w.paintRect(10, 15, 14, 16, Element.Lava)
    w.stepN(200)
    check w.count(Element.Glass) > 0

  test "liquid nitrogen freezes water into ice":
    var w = newWorld(32, 24, 11)
    discard w.paintRect(8, 16, 22, 23, Element.Water)
    w.gridMut.setElement(15, 15, Element.LiquidNitrogen)
    w.stepN(60)
    check w.count(Element.Ice) > 0

  test "liquid nitrogen quenches lava into stone":
    var w = newWorld(32, 24, 12)
    discard w.paintRect(12, 16, 18, 20, Element.Lava)
    w.gridMut.setElement(15, 15, Element.LiquidNitrogen)
    w.stepN(60)
    check w.count(Element.Stone) > 0

  test "dry ice sublimates into smoke":
    var w = newWorld(32, 24, 13)
    discard w.paintRect(12, 12, 16, 16, Element.DryIce)
    w.stepN(300)
    check w.count(Element.Smoke) > 0
    check w.count(Element.DryIce) < 25

  test "acid dissolves copper":
    var w = newWorld(32, 24, 17)
    discard w.paintRect(10, 14, 13, 19, Element.Copper)
    discard w.paintRect(10, 9, 13, 12, Element.Acid)
    let before = w.count(Element.Copper)
    w.stepN(400)
    check w.count(Element.Copper) < before

  test "gold resists acid":
    var w = newWorld(32, 24, 19)
    discard w.paintRect(10, 14, 13, 19, Element.Gold)
    discard w.paintRect(10, 9, 13, 12, Element.Acid)
    let before = w.count(Element.Gold)
    w.stepN(400)
    check w.count(Element.Gold) == before

  test "thermite burns into lava":
    var w = newWorld(32, 24, 23)
    # A single narrow column would slump sideways, so rest the thermite on the
    # floor and light it from above: fire stays fed and spreads through the pile.
    discard w.paintRect(10, 22, 14, 23, Element.Thermite)
    w.gridMut.setElement(12, 21, Element.Fire)
    w.stepN(340)
    check w.count(Element.Lava) > 0

  test "methane explodes near fire":
    var w = newWorld(32, 24, 29)
    discard w.paintRect(12, 16, 20, 20, Element.Methane)
    w.gridMut.setElement(12, 15, Element.Fire)
    w.stepN(120)
    check w.count(Element.Methane) == 0
    check jsonString(w, false).contains("CH4")

  test "catalog json lists every element":
    var w = newWorld(8, 8, 1)
    let cat = jsonString(w, true)
    check cat.len > 1000
    check cat.contains("\"name\":\"Air\"")
    check cat.contains("\"name\":\"Acetylene\"")
    check cat.contains("\"name\":\"Thermite\"")
    check cat.contains("\"name\":\"Liquid Nitrogen\"")
    check cat.contains("\"name\":\"Gold\"")
    check cat.contains("\"color\":\"#")

  test "reactions json starts empty and then records":
    var w = newWorld(8, 8, 1)
    check jsonString(w, false) == "[]"
    w.gridMut.setElement(4, 4, Element.Sodium)
    w.gridMut.setElement(4, 5, Element.Water)
    w.stepN(100)
    check jsonString(w, false).contains("2 Na")

  test "undo restores the previous state":
    var w = newWorld(32, 24, 5)
    discard w.paintCircle(10, 10, 3, Element.Sand)
    w.stepN(20)
    let before = w.hash
    check not w.canUndo
    w.snapshot()
    discard w.paintCircle(16, 4, 5, Element.Tnt)
    w.explode(16, 4, 8, Element.Smoke)
    w.stepN(10)
    check w.hash != before
    check w.canUndo
    check w.undo()
    check w.hash == before
    check not w.canUndo
    check not w.undo()

  test "clear is undoable":
    var w = newWorld(32, 24, 3)
    discard w.paintRect(4, 4, 12, 12, Element.Sand)
    check w.particleCount > 0
    w.clear()
    check w.particleCount == 0
    check w.canUndo
    check w.undo()
    check w.particleCount > 0

  test "determinism":
    proc run(seed: uint32): uint32 =
      var w = newWorld(48, 32, seed)
      discard w.paintCircle(10, 10, 6, Element.Sand)
      discard w.paintCircle(30, 10, 6, Element.Water)
      discard w.paintCircle(50, 6, 4, Element.Hydrogen)
      discard w.paintCircle(20, 30, 5, Element.Lava)
      discard w.paintCircle(40, 30, 5, Element.Oil)
      w.stepN(120)
      w.hash
    check run(2024) == run(2024)
    check run(2024) != run(2025)

  test "pixel buffer is always opaque rgba":
    var w = newWorld(48, 32, 3)
    discard w.paintCircle(10, 10, 3, Element.Steam)
    discard w.paintCircle(20, 10, 3, Element.Fire)
    w.stepN(10)
    let px = w.pixels
    check px.len == w.pixelLen
    check px.len == w.len * 4
    var i = 3
    while i < px.len:
      check px[i] == 255'u8
      i += 4

  test "pixel pointer matches the buffer":
    var w = newWorld(16, 8, 1)
    let p = w.pixelPtr
    check p != nil
    check w.pixelLen == 16 * 8 * 4
