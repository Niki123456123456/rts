use crate::BuildingKind;

pub const PEASANTS_PER_HOUSING: u32 = 4;

/// Workforce follows surviving buildings, so demolition and ownership changes
/// cannot leave a separate population balance out of sync.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Population {
    pub used: u32,
    pub capacity: u32,
}

impl BuildingKind {
    /// Only buildings with an implemented production job employ a peasant.
    pub fn peasant_cost(self) -> u32 {
        u32::from(self.produces().is_some())
    }
}

impl Population {
    pub fn from_buildings(buildings: impl IntoIterator<Item = BuildingKind>) -> Self {
        let mut population = Self::default();
        for kind in buildings {
            if kind == BuildingKind::Housing {
                population.capacity += PEASANTS_PER_HOUSING;
            } else {
                population.used += kind.peasant_cost();
            }
        }
        population
    }

    pub fn can_build(self, kind: BuildingKind, count: u32) -> bool {
        count.saturating_mul(kind.peasant_cost()) <= self.capacity.saturating_sub(self.used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_working_producers_use_peasants() {
        for kind in BuildingKind::ALL {
            let expected = u32::from(matches!(
                kind,
                BuildingKind::Quarry
                    | BuildingKind::IronMine
                    | BuildingKind::SawPit
                    | BuildingKind::DairyFarm
            ));
            assert_eq!(kind.peasant_cost(), expected, "{}", kind.name());
            assert_eq!(
                Population::from_buildings([BuildingKind::Housing, kind]),
                Population {
                    used: expected,
                    capacity: if kind == BuildingKind::Housing { 8 } else { 4 },
                }
            );
        }
    }

    #[test]
    fn no_worker_buildings_remain_buildable_without_free_capacity() {
        for population in [
            Population::default(),
            Population {
                used: 4,
                capacity: 4,
            },
            Population {
                used: 5,
                capacity: 4,
            },
        ] {
            for kind in [
                BuildingKind::Housing,
                BuildingKind::Stockpile,
                BuildingKind::Granary,
                BuildingKind::StoneWall,
                BuildingKind::TowerStairwell,
                BuildingKind::Barracks,
            ] {
                assert!(population.can_build(kind, 1));
            }
            assert!(!population.can_build(BuildingKind::Quarry, 1));
        }
        let population = Population {
            used: 1,
            capacity: 4,
        };
        assert!(population.can_build(BuildingKind::Quarry, 3));
        assert!(!population.can_build(BuildingKind::Quarry, 4));
    }
}
