//! Roads are traversable surfaces; wonders have capped settlement-wide benefits.
use crate::{BuildingKind, Obstacle, Point};
impl BuildingKind {
    pub fn is_road(self) -> bool {
        matches!(self, Self::DirtRoad | Self::StoneRoad | Self::RomanBridge)
    }
    pub fn is_wonder(self) -> bool {
        matches!(
            self,
            Self::Aqueduct | Self::Colosseum | Self::Statue | Self::Library
        )
    }
    pub fn ground_solids(self, x: f32, z: f32, r: u8) -> Vec<Obstacle> {
        if self.is_road() {
            return vec![];
        }
        if self == Self::Aqueduct {
            return [-6., -2., 2., 6.]
                .into_iter()
                .map(|dx| crate::fortifications::rect((x, z), r, (dx, 0.), 0.5, 0.8))
                .collect();
        }
        vec![self.rotated_obstacle(x, z, r)]
    }
    pub fn travel_multiplier(self) -> f32 {
        match self {
            Self::DirtRoad => 1.2,
            Self::StoneRoad => 1.5,
            Self::RomanBridge => 1.35,
            _ => 1.,
        }
    }
}
pub fn road_shape(k: BuildingKind, p: Point, r: u8) -> Obstacle {
    k.rotated_obstacle(p.0, p.1, r)
}
pub fn wonder_happiness(kinds: impl Iterator<Item = BuildingKind>) -> i32 {
    let kinds = kinds.collect::<Vec<_>>();
    [
        (BuildingKind::Statue, 3),
        (BuildingKind::Temple, 5),
        (BuildingKind::Baths, 4),
        (BuildingKind::Colosseum, 7),
    ]
    .into_iter()
    .filter(|(k, _)| kinds.contains(k))
    .map(|(_, n)| n)
    .sum()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roads_and_aqueducts_leave_passages() {
        assert!(BuildingKind::StoneRoad.ground_solids(0., 0., 0).is_empty());
        let piers = BuildingKind::Aqueduct.ground_solids(0., 0., 0);
        assert!(!piers.iter().any(|p| p.contains((0., 0.), 0.35)));
        assert!(
            BuildingKind::StoneRoad.travel_multiplier()
                > BuildingKind::DirtRoad.travel_multiplier()
        );
        assert_eq!(
            wonder_happiness([BuildingKind::Statue, BuildingKind::Statue].into_iter()),
            3
        );
    }
}
