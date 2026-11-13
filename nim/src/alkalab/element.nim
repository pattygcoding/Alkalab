# The material catalogue and its physical/chemical property table.
import color
# -----------------------------------------------------------------------------
# element - the material catalogue and its physical/chemical property table
# -----------------------------------------------------------------------------

type
  Category* {.pure.} = enum
    Empty = 0,
    Solid,
    Powder,
    Liquid,
    Gas,
    Fire

  Element* {.pure.} = enum
    Empty = 0,
    Wall,
    Stone,
    Glass,
    Wood,
    Plant,
    Metal,
    Ice,
    Tnt,
    Sand,
    Salt,
    Ash,
    Snow,
    Gunpowder,
    Carbide,
    Lime,
    Water,
    SaltWater,
    Acid,
    Oil,
    Lava,
    Mercury,
    Sodium,
    Potassium,
    Lithium,
    Calcium,
    Magnesium,
    Fire,
    Smoke,
    Steam,
    Hydrogen,
    Acetylene,
    Thermite,
    Methane,
    Coal,
    DryIce,
    LiquidNitrogen,
    Firework,
    Copper,
    Gold

const ELEMENT_COUNT* = 40

func isMobile*(c: Category): bool {.inline.} =
  c in {Category.Powder, Category.Liquid, Category.Gas, Category.Fire}

func isDisplaceable*(c: Category): bool {.inline.} =
  c in {Category.Empty, Category.Gas, Category.Fire, Category.Liquid}

func categoryName*(c: Category): cstring {.inline.} =
  case c
  of Category.Empty: "Air"
  of Category.Solid: "Solid"
  of Category.Powder: "Powder"
  of Category.Liquid: "Liquid"
  of Category.Gas: "Gas"
  of Category.Fire: "Energy"

type
  ElementProps* = object
    name*: cstring
    formula*: cstring
    desc*: cstring
    category*: Category
    density*: uint8
    color*: uint32
    speckle*: uint8
    life*: uint16
    residue*: Element
    flammable*: uint8
    burnLife*: uint16
    burnResidue*: Element
    burnVariant*: uint8
    explosive*: uint8
    wettable*: bool
    corrodible*: bool
    dispersion*: uint8
    rise*: uint8
    heat*: uint8

func base(name, formula: cstring, category: Category, density: uint8,
          color: uint32, desc: cstring): ElementProps {.inline.} =
  ElementProps(name: name, formula: formula, desc: desc, category: category,
        density: density, color: color, speckle: 0, life: 0,
        residue: Element.Empty, flammable: 0, burnLife: 0,
        burnResidue: Element.Empty, burnVariant: 0, explosive: 0,
        wettable: false, corrodible: false, dispersion: 0, rise: 0, heat: 0)

func burns(props: ElementProps, chance: uint8, life: uint16, residue: Element): ElementProps {.inline.} =
  var p = props
  p.flammable = chance
  p.burnLife = life
  p.burnResidue = residue
  p

func fluid(props: ElementProps, dispersion: uint8, rise: uint8): ElementProps {.inline.} =
  var p = props
  p.dispersion = dispersion
  p.rise = rise
  p

func corrodes(props: ElementProps): ElementProps {.inline.} =
  var p = props
  p.corrodible = true
  p

func dampens(props: ElementProps): ElementProps {.inline.} =
  var p = props
  p.wettable = true
  p

# Reserved `variant` values used by the renderer and the chemistry.
const
  VARIANT_DEFAULT* = 0'u8
  VARIANT_WET* = 1'u8
  VARIANT_HOT* = 2'u8

# The property table. PROP_TABLE[element.ord] is that element's properties.
const PROP_TABLE*: array[ELEMENT_COUNT, ElementProps] = [
  # 0 - Air. Opaque so the canvas never shows through.
  base("Air", "N2+O2", Category.Empty, 0,
       rgb(11, 13, 20), "Empty space. Everything else moves into it."),
  # 1 - Immovable bedrock: the only thing explosions cannot destroy.
  base("Wall", "Inert", Category.Solid, 255,
       rgb(78, 82, 92), "Indestructible wall. Survives every explosion."),
  # 2 - Static stone.
  ElementProps(speckle: 10,
        name: "Stone", formula: "CaCO3", category: Category.Solid, density: 255,
        color: rgb(124, 126, 132),
        desc: "Static rock. Acid etches it away, lava melts it.",
        residue: Element.Empty, burnResidue: Element.Empty, wettable: false,
        corrodible: true),
  # 3 - Glass, made from sand fused by lava.
  ElementProps(speckle: 8,
        name: "Glass", formula: "SiO2", category: Category.Solid, density: 255,
        color: rgba(176, 222, 233, 190),
        desc: "Translucent solid: sand melted by lava.",
        residue: Element.Empty, burnResidue: Element.Empty, wettable: false,
        corrodible: false),
  # 4 - Wood: structural, burns well.
  burns(corrodes(base("Wood", "C6H10O5", Category.Solid, 255,
                      rgb(134, 88, 50),
                      "Structural timber. Catches fire and leaves ash.")),
        26, 150, Element.Ash),
  # 5 - Plant: grows towards empty space when it can reach water.
  burns(corrodes(base("Plant", "C6H12O6", Category.Solid, 255,
                      rgb(58, 148, 66),
                      "Living tissue. Grows with water, burns fast.")),
        44, 60, Element.Ash),
  # 6 - Steel plate: acid dissolves it, lava melts it.
  corrodes(base("Metal", "Fe", Category.Solid, 255,
                rgb(150, 156, 168),
                "Steel plate. Melts in lava and dissolves in acid.")),
  # 7 - Ice: melts next to anything hot, and salt de-ices it.
  base("Ice", "H2O(s)", Category.Solid, 255,
       rgba(158, 214, 240, 215), "Frozen water. Melts near fire and lava."),
  # 8 - TNT: the big one.
  dampens(burns(base("TNT", "C7H5N3O6", Category.Solid, 255,
                     rgb(196, 62, 52),
                     "High explosive: detonates in a wide radius. Keep it dry."),
               255, 10, Element.Smoke)),
  # 9 - Sand: the classic powder.
  base("Sand", "SiO2", Category.Powder, 160,
       rgb(214, 190, 120), "Falls, piles at 45 degrees and sinks in water."),
  # 10 - Salt: powder that dissolves and melts ice.
  base("Salt", "NaCl", Category.Powder, 152,
       rgb(233, 233, 240), "Soluble powder. Melts ice, dissolves into brine."),
  # 11 - Ash: a light powder that floats on water.
  base("Ash", "C", Category.Powder, 96,
       rgb(112, 108, 108), "Light residue of combustion. Floats on water."),
  # 12 - Snow: powder that melts easily and floats.
  base("Snow", "H2O(s)", Category.Powder, 62,
       rgb(238, 244, 255), "Fluffy powder that floats on water and melts fast."),
  # 13 - Gunpowder: explosive unless it is wet.
  dampens(burns(base("Gunpowder", "KNO3+S+C", Category.Powder, 148,
                     rgb(74, 76, 80),
                     "Low explosive powder. Water dampens it and makes it inert."),
               255, 12, Element.Smoke)),
  # 14 - Calcium carbide: fizzes into acetylene in water.
  base("Carbide", "CaC2", Category.Powder, 166,
       rgb(104, 110, 122), "CaC2 + 2 H2O -> Ca(OH)2 + C2H2 (acetylene)."),
  # 15 - Slaked lime: residue of carbide and calcium in water.
  base("Lime", "Ca(OH)2", Category.Powder, 118,
       rgb(228, 230, 220), "Slaked lime. Steams when it touches water."),
  # 16 - Water.
  fluid(base("Water", "H2O", Category.Liquid, 128,
             rgb(38, 104, 214),
             "Flows and levels out. Puts out fire, cools lava."), 5, 0),
  # 17 - Brine: denser than water, so it sinks through it.
  fluid(base("Brine", "NaCl(aq)", Category.Liquid, 142,
             rgb(58, 128, 178),
             "Salt water. Denser than water and will not grow plants."), 4, 0),
  # 18 - Acid: dissolves metal, stone, wood and tissue; spent acid is water.
  block:
    var p = fluid(base("Acid", "HCl", Category.Liquid, 134,
                       rgb(156, 226, 72),
                       "Dissolves metal, stone and tissue. Spent acid becomes water."), 5, 0)
    p.life = 220
    p.residue = Element.Water
    p,
  # 19 - Oil: floats on water and burns fiercely.
  burns(fluid(base("Oil", "CnH2n", Category.Liquid, 92,
                   rgb(62, 48, 36),
                   "Floats on water and burns with a long flame."), 5, 0),
        112, 220, Element.Smoke),
  # 20 - Lava: melts, ignites and solidifies on contact with water.
  block:
    var p = fluid(base("Lava", "SiO2(m)", Category.Liquid, 202,
                       rgb(238, 112, 26),
                       "Molten rock: fuses sand to glass, melts stone, ignites fuel."), 3, 0)
    p.heat = 140
    p,
  # 21 - Mercury: heavy liquid metal, chemically lazy in this lab.
  fluid(base("Mercury", "Hg", Category.Liquid, 246,
             rgb(196, 202, 214),
             "Very dense liquid metal. Sinks through everything."), 3, 0),
  # 22 - Sodium: the star of the show. A powder, so it falls, and light enough
  # to float on water while it tears around the surface.
  base("Sodium", "Na", Category.Powder, 100,
       rgb(216, 214, 198),
       "Floats on water and detonates: 2 Na + 2 H2O -> 2 NaOH + H2. Keep it dry."),
  # 23 - Potassium: same group, one shell further down, even angrier.
  base("Potassium", "K", Category.Powder, 96,
       rgb(208, 204, 196),
       "Floats, ignites and explodes: 2 K + 2 H2O -> 2 KOH + H2."),
  # 24 - Lithium: light enough to float, reacts gently.
  base("Lithium", "Li", Category.Powder, 92,
       rgb(198, 208, 214),
       "Floats and fizzes steadily: 2 Li + 2 H2O -> 2 LiOH + H2."),
  # 25 - Calcium: denser than water, so it sinks while it bubbles.
  base("Calcium", "Ca", Category.Powder, 140,
       rgb(206, 206, 202),
       "Sinks and fizzes: Ca + 2 H2O -> Ca(OH)2 + H2."),
  # 26 - Magnesium: sinks, and burns with a blinding white flame.
  block:
    var p = burns(base("Magnesium", "Mg", Category.Powder, 150,
                       rgb(216, 216, 222),
                       "Sinks in water. Burns with a blinding white flame: 2 Mg + O2 -> 2 MgO."),
                  210, 220, Element.Ash)
    p.burnVariant = VARIANT_HOT
    p,
  # 27 - Fire.
  block:
    var p = fluid(base("Fire", "Plasma", Category.Fire, 3,
                       rgba(246, 142, 32, 235),
                       "Spreads to fuel, burns out into smoke, dies in water."), 1, 1)
    p.life = 60
    p.residue = Element.Smoke
    p.heat = 255
    p,
  # 28 - Smoke.
  block:
    var p = fluid(base("Smoke", "CxHy", Category.Gas, 4,
                       rgba(96, 94, 98, 150),
                       "Hot soot particles. Rises, drifts and fades."), 2, 1)
    p.speckle = 8
    p.life = 130
    p.residue = Element.Empty
    p,
  # 29 - Steam: condenses back into water.
  block:
    var p = fluid(base("Steam", "H2O(g)", Category.Gas, 6,
                       rgba(206, 214, 220, 120),
                       "Water vapour with real volume: condenses back to water."), 2, 1)
    p.speckle = 6
    p.life = 260
    p.residue = Element.Water
    p,
  # 30 - Hydrogen: light and explosive.
  block:
    var p = burns(fluid(base("Hydrogen", "H2", Category.Gas, 2,
                             rgba(126, 186, 238, 95),
                             "Lighter than air and explosive: 2 H2 + O2 -> 2 H2O."), 2, 3),
                  220, 16, Element.Water)
    p.speckle = 4
    p.explosive = 4
    p,
  # 31 - Acetylene: the carbide trick.
  block:
    var p = burns(fluid(base("Acetylene", "C2H2", Category.Gas, 5,
                             rgba(160, 150, 226, 95),
                             "From carbide + water. Burns hotter and louder than hydrogen."), 2, 3),
                  235, 14, Element.Smoke)
    p.speckle = 4
    p.explosive = 5
    p,
  # 32 - Thermite: pyrotechnic powder that burns super hot and melts to lava.
  block:
    var p = burns(base("Thermite", "Fe2O3+Al", Category.Powder, 174,
                       rgb(70, 56, 48),
                       "Iron oxide + aluminium. Burns super hot and melts into lava."),
                  255, 220, Element.Lava)
    p.speckle = 8
    p.burnVariant = VARIANT_HOT
    p,
  # 33 - Methane: marsh gas, lighter than air and explosive.
  block:
    var p = burns(fluid(base("Methane", "CH4", Category.Gas, 2,
                             rgba(150, 200, 220, 80),
                             "Marsh gas. Lighter than air and explosive: CH4 + 2 O2 -> CO2 + 2 H2O."), 2, 3),
                  255, 20, Element.Smoke)
    p.speckle = 4
    p.explosive = 4
    p,
  # 34 - Coal: solid fuel that burns long and hot.
  block:
    var p = burns(base("Coal", "C", Category.Solid, 255,
                       rgb(40, 40, 45),
                       "Solid fuel. Burns long and hot, leaving ash."),
                  40, 180, Element.Ash)
    p.speckle = 10
    p,
  # 35 - Dry ice: frozen carbon dioxide that sublimates into cold smoke.
  block:
    var p = base("Dry Ice", "CO2(s)", Category.Solid, 255,
                 rgb(210, 240, 255),
                 "Frozen carbon dioxide. Sublimates into cold smoke.")
    p.speckle = 8
    p.life = 200
    p.residue = Element.Smoke
    p,
  # 36 - Liquid nitrogen: cryogenic liquid that freezes water and boils away.
  block:
    var p = fluid(base("Liquid Nitrogen", "N2(l)", Category.Liquid, 110,
                       rgb(120, 190, 240),
                       "Cryogenic liquid. Freezes water solid and boils into gas."), 5, 0)
    p.life = 240
    p.residue = Element.Smoke
    p,
  # 37 - Firework: a powder that burns brightly in a shower of sparks.
  block:
    var p = burns(base("Firework", "KNO3+Sr+Cu", Category.Powder, 130,
                       rgb(255, 120, 60),
                       "Firework powder. Burns brightly in a shower of sparks."),
                  255, 120, Element.Ash)
    p.speckle = 12
    p.burnVariant = VARIANT_HOT
    p,
  # 38 - Copper: warm reddish metal that acid dissolves.
  corrodes(base("Copper", "Cu", Category.Solid, 255,
                rgb(196, 118, 62),
                "Warm reddish metal. Acid dissolves it into copper salts.")),
  # 39 - Gold: noble metal, untouched by acid.
  base("Gold", "Au", Category.Solid, 255,
       rgb(234, 188, 72),
       "Noble metal. Acid cannot touch it; only lava melts it."),
]

# Every element in id order, for the UI and the tests.
const ELEMENT_ALL*: array[ELEMENT_COUNT, Element] = [
  Element.Empty, Element.Wall, Element.Stone, Element.Glass, Element.Wood,
  Element.Plant, Element.Metal, Element.Ice, Element.Tnt, Element.Sand,
  Element.Salt, Element.Ash, Element.Snow, Element.Gunpowder, Element.Carbide,
  Element.Lime, Element.Water, Element.SaltWater, Element.Acid, Element.Oil,
  Element.Lava, Element.Mercury, Element.Sodium, Element.Potassium,
  Element.Lithium, Element.Calcium, Element.Magnesium, Element.Fire,
  Element.Smoke, Element.Steam, Element.Hydrogen, Element.Acetylene,
  Element.Thermite, Element.Methane, Element.Coal, Element.DryIce,
  Element.LiquidNitrogen, Element.Firework, Element.Copper, Element.Gold
]

func id*(e: Element): uint8 {.inline.} = uint8(ord(e))

func fromId*(id: uint8): Element {.inline.} =
  ## Out of range ids become Empty, so a corrupt save cannot crash the engine.
  if id < uint8(ELEMENT_COUNT): Element(id) else: Element.Empty

func props*(e: Element): ElementProps {.inline.} = PROP_TABLE[ord(e)]
func category*(e: Element): Category {.inline.} = PROP_TABLE[ord(e)].category
func density*(e: Element): uint8 {.inline.} = PROP_TABLE[ord(e)].density
func colorOf*(e: Element): uint32 {.inline.} = PROP_TABLE[ord(e)].color
func name*(e: Element): cstring {.inline.} = PROP_TABLE[ord(e)].name
func formula*(e: Element): cstring {.inline.} = PROP_TABLE[ord(e)].formula
func isMobile*(e: Element): bool {.inline.} = e.category.isMobile

func lowerAscii(c: char): char {.inline.} =
  if c in {'A'..'Z'}: char(ord(c) + 32) else: c

func byName*(name: cstring): Element =
  ## Case-insensitive lookup by display name.
  for e in ELEMENT_ALL:
    let a = PROP_TABLE[ord(e)].name
    var same = true
    var i = 0
    while a[i] != '\0' and name[i] != '\0':
      if lowerAscii(a[i]) != lowerAscii(name[i]):
        same = false
        break
      inc i
    if same and a[i] == '\0' and name[i] == '\0':
      return e
  Element.Empty

