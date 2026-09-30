//! Elements, their behavioural category and the physical/chemical property table.
//!
//! Everything the simulation needs to know about an element lives in a single
//! `const PROPS` table indexed by the element's id, so adding a new material is
//! a two line change (one enum variant, one table row) and every system -
//! movement, ignition, corrosion, rendering, the HTML palette - picks it up.

use crate::color::{rgb, rgba};

/// Broad behavioural class. This is what the movement code dispatches on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Category {
    /// Air / vacuum.
    Empty,
    /// Never moves on its own; only reactions change it.
    Solid,
    /// Falls, piles up at roughly 45 degrees and can sink through lighter fluids.
    Powder,
    /// Falls like a powder but also spreads sideways to find its level.
    Liquid,
    /// Rises, drifts sideways and eventually fades.
    Gas,
    /// Rises like a gas, ignites everything flammable it touches and burns out.
    Fire,
}

impl Category {
    /// Does this category move by itself?
    #[inline]
    pub const fn is_mobile(self) -> bool {
        matches!(
            self,
            Category::Powder | Category::Liquid | Category::Gas | Category::Fire
        )
    }

    /// Can something heavier push this category out of the way?
    #[inline]
    pub const fn is_displaceable(self) -> bool {
        matches!(
            self,
            Category::Empty | Category::Gas | Category::Fire | Category::Liquid
        )
    }

    /// Stable name, used by the UI palette.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Category::Empty => "Air",
            Category::Solid => "Solid",
            Category::Powder => "Powder",
            Category::Liquid => "Liquid",
            Category::Gas => "Gas",
            Category::Fire => "Energy",
        }
    }
}

/// Every material in the lab.
///
/// The numeric order is the id used by the wasm API (and therefore by a saved
/// scene), so new elements must be appended instead of inserted.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Element {
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
}

/// Number of distinct elements.
pub const ELEMENT_COUNT: usize = 32;

/// Static material properties.
#[derive(Clone, Copy, Debug)]
pub struct Props {
    /// Display name, e.g. `Water`.
    pub name: &'static str,
    /// Chemical formula or short symbol, e.g. `H2O`.
    pub formula: &'static str,
    /// One line description shown as a tooltip in the UI.
    pub desc: &'static str,
    /// Behaviour group.
    pub category: Category,
    /// Relative density (`0..=255`). Heavier things sink through lighter fluids.
    pub density: u8,
    /// Base colour. Translucent elements are blended at render time.
    pub color: u32,
    /// Amount of per-pixel positional speckle applied when rendering.
    pub speckle: u8,
    /// Lifetime in ticks (`0` = immortal).
    pub life: u16,
    /// What the element turns into when its lifetime runs out.
    pub residue: Element,
    /// Chance out of 255 per tick to catch fire from an adjacent heat source.
    pub flammable: u8,
    /// How long the resulting flame burns for.
    pub burn_life: u16,
    /// What the flame leaves behind when it goes out.
    pub burn_residue: Element,
    /// Variant flag handed to the flame (`VARIANT_HOT` for magnesium).
    pub burn_variant: u8,
    /// Blast radius when ignited (`0` = inert).
    pub explosive: u8,
    /// Water makes it damp, which disables ignition (gunpowder, TNT).
    pub wettable: bool,
    /// Dissolved by acid.
    pub corrodible: bool,
    /// Liquids: how far sideways a cell may travel per tick while levelling.
    pub dispersion: u8,
    /// Gases/fire: how many cells upwards per tick at full speed.
    pub rise: u8,
    /// Heat output: chance out of 255 per tick to ignite neighbouring fuel.
    pub heat: u8,
}

/// Base row; override what you need with struct update syntax, e.g.
/// `Props { dispersion: 4, ..base("Water", "H2O", Category::Liquid, 128, rgb(40, 110, 215), "..") }`.
#[allow(clippy::too_many_arguments)]
const fn base(
    name: &'static str,
    formula: &'static str,
    category: Category,
    density: u8,
    color: u32,
    desc: &'static str,
) -> Props {
    Props {
        name,
        formula,
        desc,
        category,
        density,
        color,
        speckle: 0,
        life: 0,
        residue: Element::Empty,
        flammable: 0,
        burn_life: 0,
        burn_residue: Element::Empty,
        burn_variant: 0,
        explosive: 0,
        wettable: false,
        corrodible: false,
        dispersion: 0,
        rise: 0,
        heat: 0,
    }
}

/// Mark a material as fuel: catches fire with `chance`, burns for `life` ticks
/// and leaves `residue` behind.
const fn burns(props: Props, chance: u8, life: u16, residue: Element) -> Props {
    Props {
        flammable: chance,
        burn_life: life,
        burn_residue: residue,
        ..props
    }
}

/// Give a fluid its sideways dispersion and upward speed.
const fn fluid(props: Props, dispersion: u8, rise: u8) -> Props {
    Props {
        dispersion,
        rise,
        ..props
    }
}

/// Dissolved by acid.
const fn corrodes(props: Props) -> Props {
    Props {
        corrodible: true,
        ..props
    }
}

/// Water sticks to it and makes it inert while wet.
const fn dampens(props: Props) -> Props {
    Props {
        wettable: true,
        ..props
    }
}

impl Element {
    /// Every element, in id order. Useful for the UI and for tests.
    pub const ALL: [Element; ELEMENT_COUNT] = [
        Element::Empty,
        Element::Wall,
        Element::Stone,
        Element::Glass,
        Element::Wood,
        Element::Plant,
        Element::Metal,
        Element::Ice,
        Element::Tnt,
        Element::Sand,
        Element::Salt,
        Element::Ash,
        Element::Snow,
        Element::Gunpowder,
        Element::Carbide,
        Element::Lime,
        Element::Water,
        Element::SaltWater,
        Element::Acid,
        Element::Oil,
        Element::Lava,
        Element::Mercury,
        Element::Sodium,
        Element::Potassium,
        Element::Lithium,
        Element::Calcium,
        Element::Magnesium,
        Element::Fire,
        Element::Smoke,
        Element::Steam,
        Element::Hydrogen,
        Element::Acetylene,
    ];

    /// Numeric id used across the wasm boundary.
    #[inline]
    pub const fn id(self) -> u8 {
        self as u8
    }

    /// Convert an id coming from JavaScript. Out of range ids become `Empty`.
    #[inline]
    pub fn from_id(id: u8) -> Element {
        Self::ALL
            .get(usize::from(id))
            .copied()
            .unwrap_or(Element::Empty)
    }

    /// Properties of this element.
    #[inline]
    pub const fn props(self) -> &'static Props {
        &PROPS[self as usize]
    }

    #[inline]
    pub const fn category(self) -> Category {
        PROPS[self as usize].category
    }

    #[inline]
    pub const fn density(self) -> u8 {
        PROPS[self as usize].density
    }

    #[inline]
    pub const fn color(self) -> u32 {
        PROPS[self as usize].color
    }

    #[inline]
    pub const fn name(self) -> &'static str {
        PROPS[self as usize].name
    }

    #[inline]
    pub const fn formula(self) -> &'static str {
        PROPS[self as usize].formula
    }

    /// True for powders, liquids, gases and fire.
    #[inline]
    pub const fn is_mobile(self) -> bool {
        self.category().is_mobile()
    }

    /// Case insensitive lookup by name, used by tests and tooling.
    pub fn by_name(name: &str) -> Option<Element> {
        Self::ALL
            .into_iter()
            .find(|e| e.name().eq_ignore_ascii_case(name))
    }
}

/// The property table. `PROPS[element.id()]` is that element's properties.
pub const PROPS: [Props; ELEMENT_COUNT] = [
    // 0 - Air. Opaque so the canvas never shows through; the renderer adds a
    // faint position based grain on top of it.
    base(
        "Air",
        "N2+O2",
        Category::Empty,
        0,
        rgb(11, 13, 20),
        "Empty space. Everything else moves into it.",
    ),
    // 1 - Immovable bedrock: the only thing explosions cannot destroy.
    base(
        "Wall",
        "Inert",
        Category::Solid,
        255,
        rgb(78, 82, 92),
        "Indestructible wall. Survives every explosion.",
    ),
    // 2 - Static stone.
    Props {
        speckle: 10,
        ..corrodes(base(
            "Stone",
            "CaCO3",
            Category::Solid,
            255,
            rgb(124, 126, 132),
            "Static rock. Acid etches it away, lava melts it.",
        ))
    },
    // 3 - Glass, made from sand fused by lava.
    Props {
        speckle: 8,
        ..base(
            "Glass",
            "SiO2",
            Category::Solid,
            255,
            rgba(176, 222, 233, 190),
            "Translucent solid: sand melted by lava.",
        )
    },
    // 4 - Wood: structural, burns well.
    Props {
        speckle: 12,
        ..burns(
            corrodes(base(
                "Wood",
                "C6H10O5",
                Category::Solid,
                255,
                rgb(134, 88, 50),
                "Structural timber. Catches fire and leaves ash.",
            )),
            26,
            150,
            Element::Ash,
        )
    },
    // 5 - Plant: grows towards empty space when it can reach water.
    Props {
        speckle: 14,
        ..burns(
            corrodes(base(
                "Plant",
                "C6H12O6",
                Category::Solid,
                255,
                rgb(58, 148, 66),
                "Living tissue. Grows with water, burns fast.",
            )),
            44,
            60,
            Element::Ash,
        )
    },
    // 6 - Steel plate: acid dissolves it, lava melts it.
    Props {
        speckle: 6,
        ..corrodes(base(
            "Metal",
            "Fe",
            Category::Solid,
            255,
            rgb(150, 156, 168),
            "Steel plate. Melts in lava and dissolves in acid.",
        ))
    },
    // 7 - Ice: melts next to anything hot, and salt de-ices it.
    Props {
        speckle: 8,
        ..base(
            "Ice",
            "H2O(s)",
            Category::Solid,
            255,
            rgba(158, 214, 240, 215),
            "Frozen water. Melts near fire and lava.",
        )
    },
    // 8 - TNT: the big one.
    Props {
        speckle: 10,
        explosive: 15,
        ..dampens(burns(
            base(
                "TNT",
                "C7H5N3O6",
                Category::Solid,
                255,
                rgb(196, 62, 52),
                "High explosive: detonates in a wide radius. Keep it dry.",
            ),
            255,
            10,
            Element::Smoke,
        ))
    },
    // 9 - Sand: the classic powder.
    Props {
        speckle: 14,
        ..base(
            "Sand",
            "SiO2",
            Category::Powder,
            160,
            rgb(214, 190, 120),
            "Falls, piles at 45 degrees and sinks in water.",
        )
    },
    // 10 - Salt: powder that dissolves and melts ice.
    Props {
        speckle: 10,
        ..base(
            "Salt",
            "NaCl",
            Category::Powder,
            152,
            rgb(233, 233, 240),
            "Soluble powder. Melts ice, dissolves into brine.",
        )
    },
    // 11 - Ash: a light powder that floats on water.
    Props {
        speckle: 16,
        ..base(
            "Ash",
            "C",
            Category::Powder,
            96,
            rgb(112, 108, 108),
            "Light residue of combustion. Floats on water.",
        )
    },
    // 12 - Snow: powder that melts easily and floats.
    Props {
        speckle: 10,
        ..base(
            "Snow",
            "H2O(s)",
            Category::Powder,
            62,
            rgb(238, 244, 255),
            "Fluffy powder that floats on water and melts fast.",
        )
    },
    // 13 - Gunpowder: explosive unless it is wet.
    Props {
        speckle: 10,
        explosive: 6,
        ..dampens(burns(
            base(
                "Gunpowder",
                "KNO3+S+C",
                Category::Powder,
                148,
                rgb(74, 76, 80),
                "Low explosive powder. Water dampens it and makes it inert.",
            ),
            255,
            12,
            Element::Smoke,
        ))
    },
    // 14 - Calcium carbide: fizzes into acetylene in water.
    Props {
        speckle: 10,
        ..base(
            "Carbide",
            "CaC2",
            Category::Powder,
            166,
            rgb(104, 110, 122),
            "CaC2 + 2 H2O -> Ca(OH)2 + C2H2 (acetylene).",
        )
    },
    // 15 - Slaked lime: residue of carbide and calcium in water.
    Props {
        speckle: 12,
        ..base(
            "Lime",
            "Ca(OH)2",
            Category::Powder,
            118,
            rgb(228, 230, 220),
            "Slaked lime. Steams when it touches water.",
        )
    },
    // 16 - Water.
    Props {
        speckle: 6,
        ..fluid(
            base(
                "Water",
                "H2O",
                Category::Liquid,
                128,
                rgb(38, 104, 214),
                "Flows and levels out. Puts out fire, cools lava.",
            ),
            5,
            0,
        )
    },
    // 17 - Brine: denser than water, so it sinks through it.
    Props {
        speckle: 6,
        ..fluid(
            base(
                "Brine",
                "NaCl(aq)",
                Category::Liquid,
                142,
                rgb(58, 128, 178),
                "Salt water. Denser than water and will not grow plants.",
            ),
            4,
            0,
        )
    },
    // 18 - Acid: dissolves metal, stone, wood and tissue; spent acid is water.
    Props {
        speckle: 6,
        life: 220,
        residue: Element::Water,
        ..fluid(
            base(
                "Acid",
                "HCl",
                Category::Liquid,
                134,
                rgb(156, 226, 72),
                "Dissolves metal, stone and tissue. Spent acid becomes water.",
            ),
            5,
            0,
        )
    },
    // 19 - Oil: floats on water and burns fiercely.
    Props {
        speckle: 8,
        ..burns(
            fluid(
                base(
                    "Oil",
                    "CnH2n",
                    Category::Liquid,
                    92,
                    rgb(62, 48, 36),
                    "Floats on water and burns with a long flame.",
                ),
                5,
                0,
            ),
            112,
            220,
            Element::Smoke,
        )
    },
    // 20 - Lava: melts, ignites and solidifies on contact with water.
    Props {
        speckle: 10,
        heat: 140,
        ..fluid(
            base(
                "Lava",
                "SiO2(m)",
                Category::Liquid,
                202,
                rgb(238, 112, 26),
                "Molten rock: fuses sand to glass, melts stone, ignites fuel.",
            ),
            3,
            0,
        )
    },
    // 21 - Mercury: heavy liquid metal, chemically lazy in this lab.
    Props {
        speckle: 6,
        ..fluid(
            base(
                "Mercury",
                "Hg",
                Category::Liquid,
                246,
                rgb(196, 202, 214),
                "Very dense liquid metal. Sinks through everything.",
            ),
            3,
            0,
        )
    },
    // 22 - Sodium: the star of the show. A powder, so it falls, and light enough
    // to float on water while it tears around the surface.
    Props {
        speckle: 8,
        ..base(
            "Sodium",
            "Na",
            Category::Powder,
            100,
            rgb(216, 214, 198),
            "Floats on water and detonates: 2 Na + 2 H2O -> 2 NaOH + H2. Keep it dry.",
        )
    },
    // 23 - Potassium: same group, one shell further down, even angrier.
    Props {
        speckle: 8,
        ..base(
            "Potassium",
            "K",
            Category::Powder,
            96,
            rgb(208, 204, 196),
            "Floats, ignites and explodes: 2 K + 2 H2O -> 2 KOH + H2.",
        )
    },
    // 24 - Lithium: light enough to float, reacts gently.
    Props {
        speckle: 8,
        ..base(
            "Lithium",
            "Li",
            Category::Powder,
            92,
            rgb(198, 208, 214),
            "Floats and fizzes steadily: 2 Li + 2 H2O -> 2 LiOH + H2.",
        )
    },
    // 25 - Calcium: denser than water, so it sinks while it bubbles.
    Props {
        speckle: 8,
        ..base(
            "Calcium",
            "Ca",
            Category::Powder,
            140,
            rgb(206, 206, 202),
            "Sinks and fizzes: Ca + 2 H2O -> Ca(OH)2 + H2.",
        )
    },
    // 26 - Magnesium: sinks, and burns with a blinding white flame.
    Props {
        speckle: 8,
        burn_variant: crate::cell::VARIANT_HOT,
        ..burns(
            base(
                "Magnesium",
                "Mg",
                Category::Powder,
                150,
                rgb(216, 216, 222),
                "Sinks in water. Burns with a blinding white flame: 2 Mg + O2 -> 2 MgO.",
            ),
            210,
            220,
            Element::Ash,
        )
    },
    // 27 - Fire.
    Props {
        life: 60,
        residue: Element::Smoke,
        heat: 255,
        ..fluid(
            base(
                "Fire",
                "Plasma",
                Category::Fire,
                3,
                rgba(246, 142, 32, 235),
                "Spreads to fuel, burns out into smoke, dies in water.",
            ),
            1,
            1,
        )
    },
    // 28 - Smoke.
    Props {
        speckle: 8,
        life: 130,
        residue: Element::Empty,
        ..fluid(
            base(
                "Smoke",
                "CxHy",
                Category::Gas,
                4,
                rgba(96, 94, 98, 150),
                "Hot soot particles. Rises, drifts and fades.",
            ),
            2,
            1,
        )
    },
    // 29 - Steam: condenses back into water.
    Props {
        speckle: 6,
        life: 260,
        residue: Element::Water,
        ..fluid(
            base(
                "Steam",
                "H2O(g)",
                Category::Gas,
                6,
                rgba(206, 214, 220, 120),
                "Water vapour with real volume: condenses back to water.",
            ),
            2,
            1,
        )
    },
    // 30 - Hydrogen: light and explosive.
    Props {
        speckle: 4,
        explosive: 4,
        ..burns(
            fluid(
                base(
                    "Hydrogen",
                    "H2",
                    Category::Gas,
                    2,
                    rgba(126, 186, 238, 95),
                    "Lighter than air and explosive: 2 H2 + O2 -> 2 H2O.",
                ),
                2,
                3,
            ),
            220,
            16,
            Element::Water,
        )
    },
    // 31 - Acetylene: the carbide trick.
    Props {
        speckle: 4,
        explosive: 5,
        ..burns(
            fluid(
                base(
                    "Acetylene",
                    "C2H2",
                    Category::Gas,
                    5,
                    rgba(160, 150, 226, 95),
                    "From carbide + water. Burns hotter and louder than hydrogen.",
                ),
                2,
                3,
            ),
            235,
            14,
            Element::Smoke,
        )
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::VARIANT_HOT;

    #[test]
    fn ids_match_table_order() {
        for (i, element) in Element::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(element.id()), i, "{element:?} has a bad id");
            assert_eq!(PROPS[i].name, element.name());
            assert_eq!(PROPS[i].formula, element.formula());
        }
    }

    #[test]
    fn from_id_is_total() {
        assert_eq!(Element::from_id(0), Element::Empty);
        assert_eq!(Element::from_id(Element::Water.id()), Element::Water);
        assert_eq!(Element::from_id(255), Element::Empty);
        assert_eq!(Element::from_id(ELEMENT_COUNT as u8), Element::Empty);
    }

    #[test]
    fn table_is_complete_and_unique() {
        for element in Element::ALL {
            let props = element.props();
            assert!(!props.name.is_empty(), "{element:?} has no name");
            assert!(!props.formula.is_empty(), "{element:?} has no formula");
            assert!(!props.desc.is_empty(), "{element:?} has no description");
            assert!(
                props.name.len() <= 12,
                "{element:?} name is too long for the UI"
            );
        }
        let mut names: Vec<&str> = Element::ALL.iter().map(|e| e.name()).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate element names");

        let mut colors: Vec<u32> = Element::ALL.iter().map(|e| e.color()).collect();
        colors.sort_unstable();
        let total = colors.len();
        colors.dedup();
        assert_eq!(colors.len(), total, "two elements share a colour");
    }

    #[test]
    fn well_known_formulas_are_right() {
        assert_eq!(Element::Water.formula(), "H2O");
        assert_eq!(Element::Sodium.formula(), "Na");
        assert_eq!(Element::Potassium.formula(), "K");
        assert_eq!(Element::Sand.formula(), "SiO2");
    }

    #[test]
    fn air_and_wall_are_sane() {
        assert_eq!(Element::Empty.category(), Category::Empty);
        assert_eq!(Element::Empty.density(), 0);
        assert!(!Element::Empty.is_mobile());
        assert_eq!(Element::Wall.category(), Category::Solid);
        assert!(!Element::Wall.is_mobile());
        // Wall must survive everything: no fuel, no acid target, no blast.
        assert_eq!(Element::Wall.props().flammable, 0);
        assert!(!Element::Wall.props().corrodible);
        assert_eq!(Element::Wall.props().explosive, 0);
    }

    #[test]
    fn gases_are_lighter_than_liquids_which_are_lighter_than_bedrock() {
        for gas in Element::ALL
            .iter()
            .filter(|e| matches!(e.category(), Category::Gas | Category::Fire))
        {
            assert!(
                gas.density() < Element::Water.density(),
                "{gas:?} should be lighter than water"
            );
        }
        assert!(Element::Mercury.density() > Element::Water.density());
        assert!(Element::Lava.density() > Element::Water.density());
        assert!(Element::Oil.density() < Element::Water.density());
        assert!(Element::Sand.density() > Element::Water.density());
        assert!(Element::Snow.density() < Element::Water.density());
        assert!(Element::SaltWater.density() > Element::Water.density());
    }

    #[test]
    fn transient_elements_have_a_residue_and_a_life() {
        for element in [Element::Fire, Element::Smoke, Element::Steam, Element::Acid] {
            let props = element.props();
            assert!(props.life > 0, "{element:?} must expire");
            assert_ne!(
                props.residue, element,
                "{element:?} residue loops on itself"
            );
        }
        assert_eq!(Element::Steam.props().residue, Element::Water);
        assert_eq!(Element::Acid.props().residue, Element::Water);
        assert_eq!(Element::Fire.props().residue, Element::Smoke);
        assert_eq!(Element::Smoke.props().residue, Element::Empty);
    }

    #[test]
    fn fuels_declare_a_burn_duration_and_residue() {
        for element in [
            Element::Wood,
            Element::Plant,
            Element::Oil,
            Element::Gunpowder,
            Element::Tnt,
            Element::Hydrogen,
            Element::Acetylene,
            Element::Magnesium,
        ] {
            let props = element.props();
            assert!(
                props.flammable > 0,
                "{element:?} is listed as fuel but cannot catch fire"
            );
            assert!(props.burn_life > 0, "{element:?} has no burn duration");
        }
        assert_eq!(Element::Wood.props().burn_residue, Element::Ash);
        assert_eq!(Element::Magnesium.props().burn_variant, VARIANT_HOT);
    }

    #[test]
    fn explosives_are_flammable_and_bounded() {
        for element in [
            Element::Gunpowder,
            Element::Tnt,
            Element::Hydrogen,
            Element::Acetylene,
        ] {
            let props = element.props();
            assert!(props.explosive > 0, "{element:?} should explode");
            assert!(props.explosive <= 16, "{element:?} blast radius is huge");
            assert!(
                props.flammable >= 200,
                "{element:?} must catch fire almost immediately"
            );
        }
        assert!(Element::Tnt.props().explosive > Element::Gunpowder.props().explosive);
    }

    #[test]
    fn categories_match_behaviour() {
        assert_eq!(Element::Sand.category(), Category::Powder);
        assert_eq!(Element::Snow.category(), Category::Powder);
        assert_eq!(Element::Water.category(), Category::Liquid);
        assert_eq!(Element::Mercury.category(), Category::Liquid);
        assert_eq!(Element::Steam.category(), Category::Gas);
        assert_eq!(Element::Fire.category(), Category::Fire);
        for element in [Element::Stone, Element::Metal, Element::Wood, Element::Tnt] {
            assert!(!element.is_mobile(), "{element:?} must not walk around");
        }
        for element in [Element::Water, Element::Lava, Element::Acid] {
            assert!(element.props().dispersion > 0, "{element:?} cannot spread");
        }
        for element in [Element::Steam, Element::Smoke, Element::Hydrogen] {
            assert!(element.props().rise > 0, "{element:?} cannot rise");
        }
    }

    #[test]
    fn acid_only_dissolves_what_it_should() {
        for element in [
            Element::Metal,
            Element::Stone,
            Element::Wood,
            Element::Plant,
        ] {
            assert!(element.props().corrodible, "{element:?} should dissolve");
        }
        for element in [
            Element::Wall,
            Element::Sand,
            Element::Water,
            Element::Mercury,
        ] {
            assert!(
                !element.props().corrodible,
                "{element:?} should resist acid"
            );
        }
    }

    #[test]
    fn only_gunpowder_and_tnt_get_damped_by_water() {
        assert!(Element::Gunpowder.props().wettable);
        assert!(Element::Tnt.props().wettable);
        assert!(!Element::Sand.props().wettable);
        assert!(!Element::Wood.props().wettable);
    }

    #[test]
    fn by_name_round_trips() {
        for element in Element::ALL {
            assert_eq!(Element::by_name(element.name()), Some(element));
        }
        assert_eq!(Element::by_name("water"), Some(Element::Water));
        assert_eq!(Element::by_name("unobtainium"), None);
    }
}
