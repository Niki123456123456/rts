//! Combat unit types and recruitment limits.
pub const MAX_COMBAT_UNITS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Lord,
    ArmedPeasant,
}
impl UnitKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lord" => Some(Self::Lord),
            "armed_peasant" => Some(Self::ArmedPeasant),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Lord => "lord",
            Self::ArmedPeasant => "armed_peasant",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lord => "Burgherr",
            Self::ArmedPeasant => "Bewaffneter Bauer",
        }
    }
}
