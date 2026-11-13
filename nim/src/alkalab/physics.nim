# Movement rules, plant growth and ignition.
import element, celltype, world_types, neighbours, rng, grid
# -----------------------------------------------------------------------------
# physics - movement per category, plant growth and ignition
# -----------------------------------------------------------------------------

func canSink*(mover, target: Element): bool {.inline.} =
  ## Can `mover`, moving down, push `target` out of the way?
  if target == Element.Wall or mover == Element.Wall:
    return false
  case target.category
  of Category.Empty:
    true
  of Category.Liquid, Category.Gas, Category.Fire, Category.Powder:
    mover.density > target.density
  of Category.Solid:
    false

func canRise*(mover, target: Element): bool {.inline.} =
  if target == Element.Wall or mover == Element.Wall:
    return false
  case target.category
  of Category.Empty:
    true
  of Category.Liquid, Category.Gas, Category.Fire:
    mover.density < target.density
  of Category.Powder, Category.Solid:
    false

func fallDistance*(vel: int8, max: int32): int32 {.inline.} =
  let v = int32(vel) div 6 + 1
  if v < max: v else: max

func climbDistance*(vel: int8, base: uint8): int32 {.inline.} =
  let b = if base < 1'u8: 1'i32 else: int32(base)
  let extra = if vel >= 8'i8: 1'i32 else: 0'i32
  let total = b + extra
  if total < TERMINAL_RISE: total else: TERMINAL_RISE

func randomSide(world: World): int32 {.inline.} =
  if world.rng.flip: 1'i32 else: -1'i32

proc stepPowder*(world: World, x, y: int32) =
  let c = world.grid.cell(x, y)
  let distance = fallDistance(c.vel, TERMINAL_FALL)
  var steps = 0'i32
  while steps < distance and canSink(c.element, world.grid.element(x, y + steps + 1)):
    inc steps
  if steps > 0:
    world.moveWithMomentum((x, y), (x, y + steps))
    return
  let first = randomSide(world)
  for side in [first, -first]:
    if canSink(c.element, world.grid.element(x + side, y + 1)):
      world.moveCell((x, y), (x + side, y + 1))
      world.stopMomentum(x + side, y + 1)
      return
  world.stopMomentum(x, y)

proc stepLiquid*(world: World, x, y: int32) =
  let c = world.grid.cell(x, y)
  let dispersion = if c.element.props.dispersion < 1'u8: 1'i32
                   else: int32(c.element.props.dispersion)
  let distance = fallDistance(c.vel, 2)
  var steps = 0'i32
  while steps < distance and canSink(c.element, world.grid.element(x, y + steps + 1)):
    inc steps
  if steps > 0:
    world.moveWithMomentum((x, y), (x, y + steps))
    return
  let first = randomSide(world)
  for side in [first, -first]:
    if canSink(c.element, world.grid.element(x + side, y + 1)):
      world.moveCell((x, y), (x + side, y + 1))
      world.stopMomentum(x + side, y + 1)
      return
  # Flow towards the nearest place where this liquid could fall.
  for side in [first, -first]:
    var d = 1'i32
    while d <= dispersion:
      let nx = x + side * d
      if world.grid.element(nx, y) != Element.Empty:
        break
      if canSink(c.element, world.grid.element(nx, y + 1)):
        world.moveCell((x, y), (nx, y))
        world.stopMomentum(nx, y)
        return
      inc d
  for side in [first, -first]:
    if world.grid.element(x + side, y) == Element.Empty:
      world.moveCell((x, y), (x + side, y))
      world.stopMomentum(x + side, y)
      return
  world.stopMomentum(x, y)

proc stepGas*(world: World, x, y: int32) =
  let c = world.grid.cell(x, y)
  let distance = climbDistance(c.vel, c.element.props.rise)
  var steps = 0'i32
  while steps < distance and canRise(c.element, world.grid.element(x, y - steps - 1)):
    inc steps
  if steps > 0:
    world.moveWithMomentum((x, y), (x, y - steps))
    return
  let first = randomSide(world)
  for side in [first, -first]:
    if canRise(c.element, world.grid.element(x + side, y - 1)):
      world.moveCell((x, y), (x + side, y - 1))
      world.stopMomentum(x + side, y - 1)
      return
  # Drift sideways so a plume spreads under a ceiling.
  let drift = if c.element.props.dispersion < 1'u8: 1'i32
              else: int32(c.element.props.dispersion)
  for side in [first, -first]:
    if not world.rng.chance(150):
      continue
    var reach = 0'i32
    var d = 1'i32
    while d <= drift:
      if world.grid.element(x + side * d, y) != Element.Empty:
        break
      reach = d
      inc d
    if reach > 0:
      let tx = x + side * reach
      world.moveCell((x, y), (tx, y))
      world.stopMomentum(tx, y)
      return
  world.stopMomentum(x, y)

func flamesAreFed(world: World, x, y: int32): bool =
  for d in NEIGHBOURS8:
    let neighbour = world.grid.cell(x + d[0], y + d[1])
    if neighbour.element.props.flammable > 0'u8 and not neighbour.isWet:
      return true
  false

proc stepFire*(world: World, x, y: int32) =
  if flamesAreFed(world, x, y):
    return
  if (world.frame mod 2 == 0) and canRise(Element.Fire, world.grid.element(x, y - 1)):
    world.moveCell((x, y), (x, y - 1))
    return
  let first = randomSide(world)
  for side in [first, -first]:
    if canRise(Element.Fire, world.grid.element(x + side, y - 1)):
      world.moveCell((x, y), (x + side, y - 1))
      return
  for side in [first, -first]:
    if world.grid.element(x + side, y) == Element.Empty and world.rng.chance(70):
      world.moveCell((x, y), (x + side, y))
      return

proc growPlant(world: World, x, y: int32) =
  if world.grid.element(x, y - 1) != Element.Empty or not world.rng.chance(12):
    return
  var water = false
  var neighbours = 0
  for d in NEIGHBOURS8:
    case world.grid.element(x + d[0], y + d[1])
    of Element.Water: water = true
    of Element.Plant: inc neighbours
    else: discard
  if not water or neighbours > 4:
    return
  world.grid.setElement(x, y - 1, Element.Plant)
  world.grid.mark(x, y - 1, world.frame)
  world.logReaction("6 CO2 + 6 H2O + light -> C6H12O6 + 6 O2  (growth)")

proc stepSolid*(world: World, x, y: int32) =
  if world.grid.element(x, y) == Element.Plant:
    world.growPlant(x, y)

proc igniteNeighbours*(world: World, x, y: int32, source: Element) =
  for d in NEIGHBOURS4:
    let nx = x + d[0]
    let ny = y + d[1]
    let target = world.grid.cell(nx, ny)
    let p = target.element.props
    if p.flammable == 0'u8 or target.isWet or target.element == source:
      continue
    if not world.rng.chance(p.flammable):
      continue
    if p.explosive > 0'u8:
      let equation: cstring =
        case target.element
        of Element.Tnt: "detonation: C7H5N3O6 -> CO2 + H2O + N2  (huge bang)"
        of Element.Gunpowder: "detonation: 2 KNO3 + S + 3 C -> K2S + N2 + 3 CO2"
        of Element.Hydrogen: "detonation: 2 H2 + O2 -> 2 H2O  (big bang)"
        of Element.Methane: "detonation: CH4 + 2 O2 -> CO2 + 2 H2O  (big bang)"
        else: "detonation: 2 C2H2 + 5 O2 -> 4 CO2 + 2 H2O"
      world.logReaction(equation)
      world.explode(nx, ny, int32(p.explosive), p.burnResidue)
      continue
    world.grid.set(nx, ny,
      cellRaw(Element.Fire, p.burnVariant, p.burnLife, 0'i8, p.burnResidue))
    let equation: cstring =
      case target.element
      of Element.Wood: "combustion: C6H10O5 + 6 O2 -> 6 CO2 + 5 H2O  (wood + ash)"
      of Element.Plant: "combustion: C6H12O6 + 6 O2 -> 6 CO2 + 6 H2O"
      of Element.Oil: "combustion: 2 CnH2n + 3n O2 -> 2n CO2 + 2n H2O"
      of Element.Magnesium: "2 Mg + O2 -> 2 MgO  (blinding white flame)"
      of Element.Thermite: "combustion: Fe2O3 + 2 Al -> 2 Fe + Al2O3  (thermite)"
      of Element.Coal: "combustion: C + O2 -> CO2  (coal + ash)"
      of Element.Firework: "combustion: firework -> bright sparks + ash"
      else: "combustion: fuel + O2 -> CO2 + H2O"
    world.logReaction(equation)
