//! The materials of the world and their physical properties.

/// Every kind of cell. `repr(u8)` so the browser can pass elements as small integers.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Empty = 0,
    Wall = 1,
    Sand = 2,
    Water = 3,
    Fire = 4,
    Smoke = 5,
    Steam = 6,
    Wood = 7,
    Oil = 8,
    Lava = 9,
    Acid = 10,
    Ice = 11,
    Plant = 12,
    Glass = 13,
    Stone = 14,
    /// Burning wood or plant: stays in place, glows, throws flames upward, then turns to smoke.
    Ember = 15,
}

/// How a material moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Does not move: walls, wood, ice, plants, glass.
    Solid,
    /// Falls and piles up: sand, stone.
    Powder,
    /// Falls and spreads sideways: water, oil, acid, lava.
    Liquid,
    /// Rises and drifts: smoke, steam.
    Gas,
    /// Empty space and fire: anything can move into it.
    Space,
}

impl Element {
    pub const ALL: [Element; 16] = [
        Element::Empty,
        Element::Wall,
        Element::Sand,
        Element::Water,
        Element::Fire,
        Element::Smoke,
        Element::Steam,
        Element::Wood,
        Element::Oil,
        Element::Lava,
        Element::Acid,
        Element::Ice,
        Element::Plant,
        Element::Glass,
        Element::Stone,
        Element::Ember,
    ];

    pub fn from_u8(value: u8) -> Option<Element> {
        Element::ALL.get(value as usize).copied()
    }

    pub fn kind(self) -> Kind {
        use Element::*;
        match self {
            Empty | Fire => Kind::Space,
            Wall | Wood | Ice | Plant | Glass | Ember => Kind::Solid,
            Sand | Stone => Kind::Powder,
            Water | Oil | Acid | Lava => Kind::Liquid,
            Smoke | Steam => Kind::Gas,
        }
    }

    /// Heavier elements sink through lighter fluids: sand through water, water under oil.
    pub fn density(self) -> u8 {
        use Element::*;
        match self {
            Empty => 0,
            Fire | Smoke | Steam => 1,
            Oil => 3,
            Water | Acid => 4,
            Lava => 6,
            Sand => 8,
            Stone => 9,
            Wall | Wood | Ice | Plant | Glass | Ember => u8::MAX,
        }
    }

    /// How far a liquid may flow sideways in one step.
    pub fn dispersion(self) -> usize {
        match self {
            Element::Water => 4,
            Element::Acid => 3,
            Element::Oil => 3,
            Element::Lava => 1,
            _ => 0,
        }
    }

    /// Chance (per step, out of 1000) that fire next to it catches.
    pub fn flammability(self) -> u32 {
        match self {
            Element::Oil => 350,
            Element::Plant => 40,
            Element::Wood => 12,
            _ => 0,
        }
    }

    /// How long fire from this fuel burns, in steps (min, max).
    pub fn burn_time(self) -> (u8, u8) {
        match self {
            Element::Oil => (8, 20),
            Element::Wood => (70, 140),
            Element::Plant => (15, 30),
            _ => (20, 40),
        }
    }

    /// Whether acid can eat through it.
    pub fn dissolvable(self) -> bool {
        !matches!(
            self,
            Element::Empty
                | Element::Wall
                | Element::Glass
                | Element::Acid
                | Element::Fire
                | Element::Smoke
                | Element::Steam
        )
    }

    /// Base colour as 0xRRGGBB.
    pub fn color(self) -> u32 {
        use Element::*;
        match self {
            Empty => 0x10_12_1a,
            Wall => 0x6b_6f_7a,
            Sand => 0xe2_bf_6e,
            Water => 0x2f_7d_e1,
            Fire => 0xff_8a_1c,
            Smoke => 0x55_58_60,
            Steam => 0xb8_c4_d6,
            Wood => 0x7a_4e_2a,
            Oil => 0x4a_3a_2a,
            Lava => 0xf0_4a_14,
            Acid => 0x7c_e8_3a,
            Ice => 0xa8_e4_f5,
            Plant => 0x2f_a3_4a,
            Glass => 0xc9_e8_ef,
            Stone => 0x8a_86_80,
            Ember => 0xc8_3a_10,
        }
    }

    pub fn name(self) -> &'static str {
        use Element::*;
        match self {
            Empty => "eraser",
            Wall => "wall",
            Sand => "sand",
            Water => "water",
            Fire => "fire",
            Smoke => "smoke",
            Steam => "steam",
            Wood => "wood",
            Oil => "oil",
            Lava => "lava",
            Acid => "acid",
            Ice => "ice",
            Plant => "plant",
            Glass => "glass",
            Stone => "stone",
            Ember => "ember",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_u8() {
        for element in Element::ALL {
            assert_eq!(Element::from_u8(element as u8), Some(element));
        }
        assert_eq!(Element::from_u8(200), None);
    }

    #[test]
    fn sand_sinks_and_oil_floats() {
        assert!(Element::Sand.density() > Element::Water.density());
        assert!(Element::Water.density() > Element::Oil.density());
        assert!(Element::Oil.density() > Element::Smoke.density());
    }
}
