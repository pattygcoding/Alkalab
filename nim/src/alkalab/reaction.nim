# Declarative chemistry table and the pair lookup.
import celltype, element, world_types, neighbours, rng, grid
# -----------------------------------------------------------------------------
# reaction - the declarative chemistry table and the pair lookup
# -----------------------------------------------------------------------------

type
  OutcomeKind* {.pure.} = enum
    okKeep, okEmpty, okBecome, okWeaken, okWet

  Outcome* = object
    kind*: OutcomeKind
    element*: Element
    amount*: uint8

  EffectKind* {.pure.} = enum
    ekNone, ekExplode

  Effect* = object
    kind*: EffectKind
    radius*: uint8
    gas*: Element

  Reaction* = object
    a*: Element
    b*: Element
    chance*: uint8
    outcomeA*: Outcome
    outcomeB*: Outcome
    effect*: Effect
    equation*: cstring

func oKeep*(): Outcome {.inline.} = Outcome(kind: okKeep)
func oEmpty*(): Outcome {.inline.} = Outcome(kind: okEmpty)
func oBecome*(e: Element): Outcome {.inline.} = Outcome(kind: okBecome, element: e)
func oWeaken*(n: uint8): Outcome {.inline.} = Outcome(kind: okWeaken, amount: n)
func oWet*(): Outcome {.inline.} = Outcome(kind: okWet)
func eNone*(): Effect {.inline.} = Effect(kind: ekNone)
func eExplode*(radius: uint8, gas: Element): Effect {.inline.} =
  Effect(kind: ekExplode, radius: radius, gas: gas)

func rule*(a, b: Element, chance: uint8, oa, ob: Outcome,
           equation: cstring): Reaction {.inline.} =
  Reaction(a: a, b: b, chance: chance, outcomeA: oa, outcomeB: ob,
           effect: eNone(), equation: equation)

func boom*(a, b: Element, chance: uint8, radius: uint8, gas: Element,
           oa, ob: Outcome, equation: cstring): Reaction {.inline.} =
  Reaction(a: a, b: b, chance: chance, outcomeA: oa, outcomeB: ob,
           effect: eExplode(radius, gas), equation: equation)

const REACTIONS*: array[49, Reaction] = [
  # --- Alkali and alkaline earth metals in water ---------------------------
  # 2 Na + 2 H2O -> 2 NaOH + H2. The classic demonstration.
  boom(Element.Sodium, Element.Water, 128, 6, Element.Hydrogen,
       oEmpty(), oBecome(Element.SaltWater),
       "2 Na + 2 H2O -> 2 NaOH + H2  (EXPLOSION)"),
  # Potassium is the same reaction, one shell further down the group.
  boom(Element.Potassium, Element.Water, 255, 9, Element.Hydrogen,
       oEmpty(), oBecome(Element.SaltWater),
       "2 K + 2 H2O -> 2 KOH + H2  (violent!)"),
  # Lithium is mild: it dissolves and fizzes hydrogen without catching fire.
  rule(Element.Lithium, Element.Water, 10, oEmpty(), oKeep(),
       "2 Li + 2 H2O -> 2 LiOH + H2  (gentle fizz)"),
  rule(Element.Lithium, Element.Water, 55, oKeep(), oBecome(Element.Hydrogen),
       "2 Li + 2 H2O -> 2 LiOH + H2  (hydrogen bubbles)"),
  rule(Element.Calcium, Element.Water, 10, oBecome(Element.Lime), oKeep(),
       "Ca + 2 H2O -> Ca(OH)2 + H2  (slaked lime)"),
  rule(Element.Calcium, Element.Water, 30, oKeep(), oBecome(Element.Hydrogen),
       "Ca + 2 H2O -> Ca(OH)2 + H2  (bubbles)"),
  rule(Element.Magnesium, Element.Water, 2, oBecome(Element.Lime), oKeep(),
       "Mg + 2 H2O -> Mg(OH)2 + H2  (barely reacts)"),
  rule(Element.Magnesium, Element.Water, 4, oKeep(), oBecome(Element.Hydrogen),
       "Mg + 2 H2O -> Mg(OH)2 + H2  (barely reacts)"),
  # Carbide + water: the carbide cannon trick.
  rule(Element.Carbide, Element.Water, 60, oBecome(Element.Lime), oKeep(),
       "CaC2 + 2 H2O -> Ca(OH)2 + C2H2"),
  rule(Element.Carbide, Element.Water, 30, oKeep(), oBecome(Element.Acetylene),
       "CaC2 + 2 H2O -> Ca(OH)2 + C2H2  (fuel gas)"),
  # --- Metals in acid ------------------------------------------------------
  boom(Element.Sodium, Element.Acid, 255, 8, Element.Hydrogen,
       oEmpty(), oBecome(Element.Hydrogen),
       "2 Na + 2 HCl -> 2 NaCl + H2  (violent!)"),
  boom(Element.Potassium, Element.Acid, 255, 10, Element.Hydrogen,
       oEmpty(), oBecome(Element.Hydrogen),
       "2 K + 2 HCl -> 2 KCl + H2  (violent!)"),
  rule(Element.Acid, Element.Metal, 45, oKeep(), oEmpty(),
       "Fe + 2 HCl -> FeCl2 + H2  (steel dissolved)"),
  rule(Element.Acid, Element.Metal, 20, oWeaken(3), oKeep(),
       "acid spent: Fe + 2 HCl -> FeCl2 + H2"),
  rule(Element.Acid, Element.Metal, 6, oBecome(Element.Hydrogen), oKeep(),
       "Fe + 2 HCl -> FeCl2 + H2  (fizzing)"),
  # --- Acid on stone, timber and tissue ------------------------------------
  rule(Element.Acid, Element.Stone, 30, oKeep(), oEmpty(),
       "CaCO3 + 2 HCl -> CaCl2 + H2O + CO2  (stone etched)"),
  rule(Element.Acid, Element.Stone, 15, oWeaken(2), oKeep(),
       "acid spent: CaCO3 + 2 HCl -> CaCl2 + H2O + CO2"),
  rule(Element.Acid, Element.Wood, 8, oWeaken(2), oBecome(Element.Ash),
       "acid chars timber (cellulose -> carbon)"),
  rule(Element.Acid, Element.Plant, 25, oWeaken(2), oEmpty(),
       "acid dissolves tissue"),
  rule(Element.Acid, Element.Lime, 40, oWeaken(2), oEmpty(),
       "Ca(OH)2 + 2 HCl -> CaCl2 + 2 H2O"),
  rule(Element.Acid, Element.Water, 6, oBecome(Element.Water), oKeep(),
       "acid diluted: HCl + H2O -> (dilute)"),
  # --- Lava: quenched by water, melts and fuses things ---------------------
  rule(Element.Lava, Element.Water, 255, oBecome(Element.Stone),
       oBecome(Element.Steam), "lava + H2O -> stone + steam  (quenched)"),
  rule(Element.Lava, Element.Ice, 200, oKeep(), oBecome(Element.Water),
       "ice melts on lava"),
  rule(Element.Lava, Element.Ice, 40, oBecome(Element.Stone), oKeep(),
       "lava chilled by ice -> stone"),
  rule(Element.Lava, Element.Snow, 255, oKeep(), oBecome(Element.Water),
       "snow melts on lava"),
  rule(Element.Lava, Element.Sand, 30, oKeep(), oBecome(Element.Glass),
       "SiO2 (sand) + heat -> glass"),
  rule(Element.Lava, Element.Stone, 4, oKeep(), oBecome(Element.Lava),
       "stone melts into lava"),
  rule(Element.Lava, Element.Metal, 8, oKeep(), oBecome(Element.Lava),
       "steel melts into lava"),
  # --- Fire and water ------------------------------------------------------
  rule(Element.Fire, Element.Water, 255, oEmpty(), oKeep(),
       "H2O extinguishes the flame"),
  rule(Element.Fire, Element.Water, 90, oKeep(), oBecome(Element.Steam),
       "H2O + heat -> steam"),
  rule(Element.Fire, Element.Ice, 200, oKeep(), oBecome(Element.Water),
       "ice melts in the flame"),
  rule(Element.Fire, Element.Snow, 200, oKeep(), oBecome(Element.Water),
       "snow melts in the flame"),
  # --- Soluble salts, snow and dampening -----------------------------------
  rule(Element.Salt, Element.Water, 30, oEmpty(), oBecome(Element.SaltWater),
       "NaCl(s) -> Na+ + Cl-  (dissolved)"),
  rule(Element.Salt, Element.Ice, 60, oKeep(), oBecome(Element.Water),
       "salt melts ice (de-icing)"),
  rule(Element.Snow, Element.Water, 30, oBecome(Element.Water), oKeep(),
       "snow melts in water"),
  rule(Element.Gunpowder, Element.Water, 128, oWet(), oKeep(),
       "gunpowder dampened -> inert"),
  rule(Element.Tnt, Element.Water, 128, oWet(), oKeep(),
       "TNT dampened -> inert"),
  # --- Plants and lime -----------------------------------------------------
  rule(Element.Plant, Element.Water, 4, oKeep(), oEmpty(),
       "roots drink the water (transpiration)"),
  rule(Element.Lime, Element.Water, 20, oKeep(), oBecome(Element.Steam),
       "CaO + H2O -> Ca(OH)2 + heat  (slaking)"),
  rule(Element.Lime, Element.Water, 8, oBecome(Element.Stone), oKeep(),
       "slaked lime sets hard into stone"),
  # --- Cryogenics: liquid nitrogen and dry ice -----------------------------
  rule(Element.LiquidNitrogen, Element.Water, 255, oKeep(), oBecome(Element.Ice),
       "liquid nitrogen freezes water -> ice"),
  rule(Element.LiquidNitrogen, Element.Lava, 255, oBecome(Element.Smoke),
       oBecome(Element.Stone), "liquid nitrogen quenches lava -> stone + steam"),
  rule(Element.DryIce, Element.Water, 255, oBecome(Element.Smoke),
       oBecome(Element.Ice), "dry ice freezes water -> ice + cold smoke"),
  rule(Element.DryIce, Element.Lava, 255, oBecome(Element.Smoke),
       oBecome(Element.Stone), "dry ice quenches lava -> stone + smoke"),
  # --- Acid on copper: it dissolves into green copper salts ----------------
  rule(Element.Acid, Element.Copper, 45, oKeep(), oEmpty(),
       "Cu + 2 HCl -> CuCl2 + H2  (copper dissolved)"),
  rule(Element.Acid, Element.Copper, 20, oWeaken(3), oKeep(),
       "acid spent: Cu + 2 HCl -> CuCl2 + H2"),
  rule(Element.Acid, Element.Copper, 6, oBecome(Element.Hydrogen), oKeep(),
       "Cu + 2 HCl -> CuCl2 + H2  (fizzing)"),
  # --- Lava melts the metals -----------------------------------------------
  rule(Element.Lava, Element.Copper, 8, oKeep(), oBecome(Element.Lava),
       "copper melts into lava"),
  rule(Element.Lava, Element.Gold, 8, oKeep(), oBecome(Element.Lava),
       "gold melts into lava"),
]

# Lazily built pair lookup: slot[a * ELEMENT_COUNT + b] lists every rule that
# fires when a sits next to b, with a flag marking the reversed registration.
const SWAPPED* = 0x8000'u16

type
  Lookup* = object
    slots*: seq[seq[uint16]]

var
  lookupCache: Lookup
  lookupBuilt = false

func slotOf(a, b: Element): int {.inline.} =
  int(a.id) * ELEMENT_COUNT + int(b.id)

proc buildLookup(): Lookup =
  result.slots = newSeq[seq[uint16]](ELEMENT_COUNT * ELEMENT_COUNT)
  for i in 0..<REACTIONS.len:
    let idx = uint16(i)
    result.slots[slotOf(REACTIONS[i].a, REACTIONS[i].b)].add(idx)
    result.slots[slotOf(REACTIONS[i].b, REACTIONS[i].a)].add(idx or SWAPPED)

proc ensureLookup() =
  if not lookupBuilt:
    lookupCache = buildLookup()
    lookupBuilt = true

func forPair*(l: Lookup, a, b: Element): seq[uint16] {.inline.} =
  l.slots[slotOf(a, b)]

proc apply(world: World, pos: (int32, int32), expected: Element,
           outcome: Outcome): bool =
  ## Applies one outcome. Returns true when the cell became a different
  ## element (so the caller skips this cell's movement phase).
  let (x, y) = pos
  let c = world.grid.cell(x, y)
  if c.element != expected:
    return false
  case outcome.kind
  of okKeep:
    false
  of okEmpty:
    world.grid.set(x, y, cellEmpty())
    true
  of okBecome:
    world.grid.set(x, y, newCell(outcome.element))
    true
  of okWet:
    if c.element.props.wettable and c.variant != VARIANT_WET:
      world.grid.set(x, y,
        cellRaw(c.element, VARIANT_WET, c.life, c.vel, c.residue))
      true
    else:
      false
  of okWeaken:
    var cc = c
    let amount = int32(outcome.amount)
    let remaining = int32(cc.life) - amount
    cc.life = if remaining <= 0: 0'u16 else: uint16(remaining)
    if cc.life == 0:
      # Spent acid reverts to its residue (water).
      world.grid.set(x, y, newCell(cc.residue))
    else:
      world.grid.set(x, y, cc)
    false

proc react*(world: World, x, y: int32, element: Element): bool =
  ## Tries every rule that applies to the cell at (x, y). Returns true when the
  ## cell itself changed, in which case the caller skips its movement phase.
  if element == Element.Empty:
    return false
  ensureLookup()
  var selfChanged = false
  for d in NEIGHBOURS8:
    let nx = x + d[0]
    let ny = y + d[1]
    let other = world.grid.element(nx, ny)
    if other == Element.Empty:
      continue
    for entry in lookupCache.forPair(element, other):
      let reaction = REACTIONS[int(entry and not SWAPPED)]
      if not world.rng.chance(reaction.chance):
        continue
      let swapped = (entry and SWAPPED) != 0'u16
      let aPos = if swapped: (nx, ny) else: (x, y)
      let bPos = if swapped: (x, y) else: (nx, ny)
      # Consumption guard: both cells must still be what the rule expects.
      if world.grid.element(aPos[0], aPos[1]) != reaction.a or
         world.grid.element(bPos[0], bPos[1]) != reaction.b:
        continue
      let aChanged = world.apply(aPos, reaction.a, reaction.outcomeA)
      let bChanged = world.apply(bPos, reaction.b, reaction.outcomeB)
      if reaction.effect.kind == ekExplode:
        world.explode(aPos[0], aPos[1], int32(reaction.effect.radius),
                      reaction.effect.gas)
      let fired = aChanged or bChanged or reaction.effect.kind != ekNone
      if fired:
        world.logReaction(reaction.equation)
      if (if swapped: bChanged else: aChanged):
        selfChanged = true
  selfChanged

