//! Combat unit types and recruitment limits.
pub const MAX_COMBAT_UNITS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Lord,
    ArmedPeasant,
    Archer,
    LightSwordsman,
    LightCavalry,
}
impl UnitKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lord" => Some(Self::Lord),
            "armed_peasant" => Some(Self::ArmedPeasant),
            "archer" => Some(Self::Archer),
            "light_swordsman" => Some(Self::LightSwordsman),
            "light_cavalry" => Some(Self::LightCavalry),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Lord => "lord",
            Self::ArmedPeasant => "armed_peasant",
            Self::Archer => "archer",
            Self::LightSwordsman => "light_swordsman",
            Self::LightCavalry => "light_cavalry",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lord => "Burgherr",
            Self::ArmedPeasant => "Bewaffneter Bauer",
            Self::Archer => "Bogenschütze",
            Self::LightSwordsman => "Leichter Schwertkämpfer",
            Self::LightCavalry => "Leichter Reiter",
        }
    }
}

impl UnitKind {
    pub fn cost(self) -> crate::Cost {
        match self {
            Self::Lord => crate::Cost::new(0, 0, 0),
            Self::ArmedPeasant => crate::Cost::new(1, 0, 0),
            Self::Archer => crate::Cost::new(6, 0, 1),
            Self::LightSwordsman => crate::Cost::new(2, 0, 3),
            Self::LightCavalry => crate::Cost::new(8, 0, 4),
        }
    }
}

impl UnitKind {
    pub fn max_health(self) -> f32 {
        if self == Self::Lord {
            5.0
        } else {
            1.0
        }
    }
}

impl UnitKind {
    pub fn speed(self) -> f32 {
        if self == Self::LightCavalry {
            5.4
        } else {
            3.0
        }
    }
    pub fn melee_dps(self) -> f32 {
        match self {
            Self::Lord => 0.22,
            Self::LightSwordsman => 0.24,
            Self::LightCavalry => 0.28,
            _ => 0.14,
        }
    }
}
