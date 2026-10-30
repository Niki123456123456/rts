//! Combat unit types and recruitment limits.
pub const MAX_COMBAT_UNITS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Lord,
    ArmedPeasant,
    Archer,
    LightSwordsman,
    LightCavalry,
    Ballista,
    Onager,
}
impl UnitKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lord" => Some(Self::Lord),
            "armed_peasant" => Some(Self::ArmedPeasant),
            "archer" => Some(Self::Archer),
            "light_swordsman" => Some(Self::LightSwordsman),
            "light_cavalry" => Some(Self::LightCavalry),
            "ballista" => Some(Self::Ballista),
            "onager" => Some(Self::Onager),
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
            Self::Ballista => "ballista",
            Self::Onager => "onager",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lord => "Burgherr",
            Self::ArmedPeasant => "Bewaffneter Bauer",
            Self::Archer => "Bogenschütze",
            Self::LightSwordsman => "Leichter Schwertkämpfer",
            Self::LightCavalry => "Leichter Reiter",
            Self::Ballista => "Balliste",
            Self::Onager => "Onager",
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
            Self::Ballista => crate::Cost::new(20, 0, 8),
            Self::Onager => crate::Cost::new(28, 8, 10),
        }
    }
}

impl UnitKind {
    pub fn max_health(self) -> f32 {
        if self == Self::Ballista {
            return 2.5;
        }
        if self == Self::Onager {
            return 3.5;
        }
        if self == Self::Lord {
            5.0
        } else {
            1.0
        }
    }
}

impl UnitKind {
    pub fn speed(self) -> f32 {
        if self == Self::Ballista {
            return 1.3;
        }
        if self == Self::Onager {
            return 0.9;
        }
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

impl UnitKind {
    pub fn is_siege(self) -> bool {
        matches!(self, Self::Ballista | Self::Onager)
    }
    pub fn ranged_range(self, y: f32) -> f32 {
        match self {
            Self::Ballista => 34. + (y - 0.75).max(0.) * 0.4,
            Self::Onager => 42. + (y - 0.75).max(0.) * 0.3,
            _ => crate::ranged::bow_range(y),
        }
    }
    pub fn shot_interval(self) -> f32 {
        match self {
            Self::Ballista => 4.,
            Self::Onager => 8.,
            _ => crate::ranged::SHOT_INTERVAL,
        }
    }
    pub fn shot_damage(self, building: bool) -> f32 {
        match self {
            Self::Ballista => {
                if building {
                    0.18
                } else {
                    0.65
                }
            }
            Self::Onager => {
                if building {
                    0.45
                } else {
                    0.55
                }
            }
            _ => {
                if building {
                    0.06
                } else {
                    crate::ranged::SHOT_DAMAGE
                }
            }
        }
    }
}
