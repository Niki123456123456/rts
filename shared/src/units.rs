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
    StoneBallista,
    Scorpion,
    Ram,
    CoveredRam,
    SiegeTower,
    AssaultLadder,
    Pluteus,
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
            "stone_ballista" => Some(Self::StoneBallista),
            "scorpion" => Some(Self::Scorpion),
            "ram" => Some(Self::Ram),
            "covered_ram" => Some(Self::CoveredRam),
            "siege_tower" => Some(Self::SiegeTower),
            "assault_ladder" => Some(Self::AssaultLadder),
            "pluteus" => Some(Self::Pluteus),
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
            Self::StoneBallista => "stone_ballista",
            Self::Scorpion => "scorpion",
            Self::Ram => "ram",
            Self::CoveredRam => "covered_ram",
            Self::SiegeTower => "siege_tower",
            Self::AssaultLadder => "assault_ladder",
            Self::Pluteus => "pluteus",
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
            Self::StoneBallista => "Steinballista",
            Self::Scorpion => "Skorpion",
            Self::Ram => "Einfacher Rammbock",
            Self::CoveredRam => "Überdachter Rammbock",
            Self::SiegeTower => "Belagerungsturm",
            Self::AssaultLadder => "Sturmleiter",
            Self::Pluteus => "Pluteus · Schutzschirm",
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
            Self::StoneBallista => crate::Cost::new(26, 6, 10),
            Self::Scorpion => crate::Cost::new(14, 0, 5),
            Self::Ram => crate::Cost::new(16, 0, 4),
            Self::CoveredRam => crate::Cost::new(28, 2, 8),
            Self::SiegeTower => crate::Cost::new(36, 4, 8),
            Self::AssaultLadder => crate::Cost::new(6, 0, 0),
            Self::Pluteus => crate::Cost::new(12, 0, 2),
        }
    }
}

impl UnitKind {
    pub fn max_health(self) -> f32 {
        match self {
            Self::StoneBallista => return 4.0,
            Self::Scorpion => return 2.0,
            Self::Ram => return 5.0,
            Self::CoveredRam => return 8.0,
            Self::SiegeTower => return 8.0,
            Self::AssaultLadder => return 1.8,
            Self::Pluteus => return 4.0,
            _ => {}
        }
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
        match self {
            Self::StoneBallista => return 0.9,
            Self::Scorpion => return 1.6,
            Self::Ram => return 1.1,
            Self::CoveredRam => return 0.9,
            Self::SiegeTower => return 0.65,
            Self::AssaultLadder => return 2.0,
            Self::Pluteus => return 1.2,
            _ => {}
        }
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
        matches!(
            self,
            Self::Ballista
                | Self::Onager
                | Self::StoneBallista
                | Self::Scorpion
                | Self::Ram
                | Self::CoveredRam
                | Self::SiegeTower
                | Self::AssaultLadder
                | Self::Pluteus
        )
    }
    pub fn ranged_range(self, y: f32) -> f32 {
        match self {
            Self::Ballista => 34. + (y - 0.75).max(0.) * 0.4,
            Self::Onager => 42. + (y - 0.75).max(0.) * 0.3,
            Self::StoneBallista => 38.0 + (y - 0.75).max(0.) * 0.3,
            Self::Scorpion => 30.0 + (y - 0.75).max(0.) * 0.3,
            _ => crate::ranged::bow_range(y),
        }
    }
    pub fn shot_interval(self) -> f32 {
        match self {
            Self::Ballista => 4.,
            Self::Onager => 8.,
            Self::StoneBallista => 6.0,
            Self::Scorpion => 2.5,
            _ => crate::ranged::SHOT_INTERVAL,
        }
    }
    pub fn shot_damage(self, building: bool) -> f32 {
        match self {
            Self::StoneBallista => {
                if building {
                    0.34
                } else {
                    0.55
                }
            }
            Self::Scorpion => {
                if building {
                    0.06
                } else {
                    0.4
                }
            }
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

impl UnitKind {
    pub const SIEGE: [Self; 9] = [
        Self::Ballista,
        Self::StoneBallista,
        Self::Scorpion,
        Self::Onager,
        Self::Ram,
        Self::CoveredRam,
        Self::SiegeTower,
        Self::AssaultLadder,
        Self::Pluteus,
    ];
    pub fn is_ranged(self) -> bool {
        matches!(
            self,
            Self::Archer | Self::Ballista | Self::StoneBallista | Self::Scorpion | Self::Onager
        )
    }
    pub fn is_support(self) -> bool {
        matches!(self, Self::SiegeTower | Self::AssaultLadder | Self::Pluteus)
    }
    pub fn crew(self) -> u32 {
        match self {
            Self::StoneBallista => 2,
            Self::Scorpion => 1,
            Self::Ram => 2,
            Self::CoveredRam => 3,
            Self::SiegeTower => 3,
            Self::AssaultLadder => 1,
            Self::Pluteus => 1,
            _ => 2,
        }
    }
    pub fn manufacture_seconds(self) -> u32 {
        match self {
            Self::StoneBallista => 45,
            Self::Scorpion => 25,
            Self::Ram => 25,
            Self::CoveredRam => 40,
            Self::SiegeTower => 50,
            Self::AssaultLadder => 12,
            Self::Pluteus => 20,
            Self::Onager => 50,
            _ => 30,
        }
    }
    pub fn building_dps(self) -> f32 {
        match self {
            Self::Ram => 0.22,
            Self::CoveredRam => 0.32,
            Self::SiegeTower | Self::AssaultLadder | Self::Pluteus => 0.,
            _ => 0.05,
        }
    }
    pub fn description(self) -> &'static str {
        match self{
 Self::Ballista=>"Bolzen gegen Einheiten und Gebäude",
 Self::StoneBallista=>"Schwere Steinkugeln gegen Mauern · Waffenforschung 1",
 Self::Scorpion=>"Schnelle präzise Bolzen gegen Soldaten",
 Self::Onager=>"Hoher Wurfbogen, Flächenschaden, besonders wirksam gegen Mauern",
 Self::Ram=>"Direkter Angriff auf Tore und Mauern",
 Self::CoveredRam=>"Stärkerer Rammbock mit schützendem Dach · Waffenforschung 1",
 Self::SiegeTower=>"An feindliche Mauern fahren, dann eigene Fußtruppen hinaufschicken · Waffenforschung 1",
 Self::AssaultLadder=>"An feindlichen Wehrgängen bis 8,5 Meter Höhe anlegen",
 Self::Pluteus=>"Fahrbarer Schutzschirm; sein Körper fängt Geschosse ab",
 _=>""}
    }
}
