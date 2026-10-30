//! Combat unit types and recruitment limits.
pub const MAX_COMBAT_UNITS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Lord,
    ArmedPeasant,
    Archer,
}
impl UnitKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lord" => Some(Self::Lord),
            "armed_peasant" => Some(Self::ArmedPeasant),
            "archer" => Some(Self::Archer),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Lord => "lord",
            Self::ArmedPeasant => "armed_peasant",
            Self::Archer => "archer",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lord => "Burgherr",
            Self::ArmedPeasant => "Bewaffneter Bauer",
            Self::Archer => "Bogenschütze",
        }
    }
}

impl UnitKind {
    pub fn cost(self) -> crate::Cost {
        match self {
            Self::Lord => crate::Cost::new(0, 0, 0),
            Self::ArmedPeasant => crate::Cost::new(2, 0, 2),
            Self::Archer => crate::Cost::new(6, 0, 1),
        }
    }
}
