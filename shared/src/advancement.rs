//! Equipment stocks and technology rules shared by authority and UI.
use crate::{units::UnitKind, Cost};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Equipment {
    Bow,
    Sword,
    Shield,
}
impl Equipment {
    pub const ALL: [Self; 3] = [Self::Bow, Self::Sword, Self::Shield];
    pub fn name(self) -> &'static str {
        match self {
            Self::Bow => "weapon_bow",
            Self::Sword => "weapon_sword",
            Self::Shield => "weapon_shield",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Bow => "Bogen",
            Self::Sword => "Schwert",
            Self::Shield => "Schild",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == s)
    }
    pub fn cost(self) -> Cost {
        match self {
            Self::Bow => Cost::new(6, 0, 1),
            Self::Sword => Cost::new(2, 0, 2),
            Self::Shield => Cost::new(0, 0, 1),
        }
    }
    pub fn seconds(self) -> u32 {
        match self {
            Self::Bow => 12,
            Self::Sword => 12,
            Self::Shield => 8,
        }
    }
    pub fn required(self, k: UnitKind) -> bool {
        match self {
            Self::Bow => k == UnitKind::Archer,
            Self::Sword | Self::Shield => {
                matches!(k, UnitKind::LightSwordsman | UnitKind::LightCavalry)
            }
        }
    }
}
pub const BRANCHES: [&str; 3] = ["weapons", "economy", "politics"];
pub fn branch_label(s: &str) -> &'static str {
    match s {
        "weapons" => "Waffen",
        "economy" => "Wirtschaft",
        _ => "Politiken",
    }
}
pub fn research_label(s: &str, level: u32) -> &'static str {
    match (s, level) {
        ("weapons", 1) => "Belagerungsbau",
        ("weapons", 2) => "Gehärtete Waffen",
        ("weapons", _) => "Artillerieoptimierung",
        ("economy", 1) => "Bessere Werkzeuge",
        ("economy", 2) => "Arbeitsteilung",
        ("economy", _) => "Verbesserte Fertigung",
        ("politics", 1) => "Bürgerverwaltung",
        ("politics", 2) => "Provinzverwaltung",
        _ => "Römisches Bürgerrecht",
    }
}
pub fn research_effect(s: &str) -> &'static str {
    match s{"weapons"=>"Je Stufe +15 % Schaden. Stufe 1 schaltet Steinballista, überdachten Rammbock und Belagerungsturm frei.","economy"=>"Je Stufe +20 % Arbeitsfortschritt in Rohstoff- und Lebensmittelbetrieben sowie der Fabrica.",_=>"Je Stufe +10 % Steuereinnahmen und +2 Zufriedenheit."}
}
pub fn research_cost(level: u32) -> Cost {
    Cost::new(4 * level, 4 * level, 2 * level)
}
pub fn research_gold(level: u32) -> u64 {
    20 * u64::from(level)
}
pub fn research_seconds(level: u32) -> u32 {
    15 + 15 * level
}
pub fn damage(base: f32, level: u32) -> f32 {
    base * (1. + 0.15 * level.min(3) as f32)
}
pub fn taxes(base: u64, level: u32) -> u64 {
    base.saturating_mul(100 + 10 * u64::from(level.min(3))) / 100
}
pub fn advanced(k: UnitKind) -> bool {
    matches!(
        k,
        UnitKind::StoneBallista | UnitKind::CoveredRam | UnitKind::SiegeTower
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equipment_and_upgrade_effects_are_bounded() {
        assert_eq!(Equipment::Bow.cost().wood, 6);
        assert!(Equipment::Sword.required(UnitKind::LightCavalry));
        assert!(!Equipment::Shield.required(UnitKind::Archer));
        assert_eq!(taxes(100, 3), 130);
        assert!((damage(1., 3) - 1.45).abs() < 0.001);
        for k in UnitKind::SIEGE {
            assert!(k.is_siege());
            assert!(k.crew() > 0);
            assert!(k.manufacture_seconds() > 0);
        }
    }
}
