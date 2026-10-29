//! One authoritative, bounded needs model shared by simulation and UI tests.
#[derive(Clone, Copy, Default)]
pub struct NeedsInput {
    pub food: [u32; 4],
    pub goods: [u32; 3],
    pub population: u32,
    pub capacity: u32,
    pub employed: u32,
    pub workplaces: u32,
    pub water: bool,
    pub healthcare: bool,
    pub leisure: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Satisfaction {
    pub happiness: u32,
    pub food: i32,
    pub variety: i32,
    pub housing: i32,
    pub water: i32,
    pub health: i32,
    pub leisure: i32,
    pub goods: i32,
    pub employment: i32,
    pub immigration_seconds: u32,
}
impl Satisfaction {
    pub fn calculate(input: NeedsInput) -> Self {
        let stage = NeedsStage::for_population(input.population);
        let stock: u64 = input.food.into_iter().map(u64::from).sum();
        // The first four settlers give the player time to build a food economy.
        let food = if stock == 0 {
            if input.population <= 4 {
                0
            } else {
                -25
            }
        } else if stock >= u64::from(input.population.max(1)) * 2 {
            15
        } else {
            5
        };
        let types = input.food.into_iter().filter(|amount| *amount > 0).count() as i32;
        let variety = if stage.variety {
            if types < 2 {
                -4
            } else {
                (types - 1) * 4
            }
        } else {
            0
        };
        let housing = if input.population > input.capacity {
            -20
        } else if input.capacity.saturating_sub(input.population) >= 2 {
            10
        } else {
            5
        };
        let water = if !stage.water {
            0
        } else if input.water {
            8
        } else {
            -8
        };
        let health = if !stage.health {
            0
        } else if input.healthcare {
            5
        } else {
            -5
        };
        let leisure = if !stage.leisure {
            0
        } else if input.leisure == 0 {
            -5
        } else {
            input.leisure.min(2) as i32 * 5
        };
        let goods = if stage.goods {
            input
                .goods
                .into_iter()
                .filter(|amount| *amount >= 5)
                .count() as i32
                * 2
        } else {
            0
        };
        let unemployed = input.population.saturating_sub(input.employed);
        let employment = if !stage.employment {
            0
        } else if unemployed > 4 {
            -8
        } else if input.workplaces > input.employed {
            4
        } else if input.employed > 0 {
            3
        } else {
            0
        };
        let happiness =
            (stage.base + food + variety + housing + water + health + leisure + goods + employment)
                .clamp(0, 100) as u32;
        let immigration_seconds = match happiness {
            80..=100 => 4,
            65..=79 => 8,
            50..=64 => 14,
            _ => 0,
        };
        Self {
            happiness,
            food,
            variety,
            housing,
            water,
            health,
            leisure,
            goods,
            employment,
            immigration_seconds,
        }
    }
}
/// Population thresholds are shared by the authoritative score and its UI.
#[derive(Clone, Copy)]
pub struct NeedsStage {
    pub base: i32,
    pub water: bool,
    pub variety: bool,
    pub employment: bool,
    pub health: bool,
    pub leisure: bool,
    pub goods: bool,
}
impl NeedsStage {
    pub fn for_population(population: u32) -> Self {
        Self {
            base: match population {
                0..=7 => 75,
                8..=15 => 65,
                16..=23 => 55,
                _ => 50,
            },
            water: population >= 8,
            variety: population >= 16,
            employment: population >= 16,
            health: population >= 24,
            leisure: population >= 32,
            goods: population >= 40,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_four_settlers_are_happy_without_advanced_services_or_food() {
        for population in 0..=4 {
            let s = Satisfaction::calculate(NeedsInput {
                population,
                capacity: 4,
                ..Default::default()
            });
            assert!(s.happiness >= 80);
            assert_eq!(s.immigration_seconds, 4);
            assert_eq!(
                (
                    s.food,
                    s.water,
                    s.variety,
                    s.health,
                    s.leisure,
                    s.goods,
                    s.employment
                ),
                (0, 0, 0, 0, 0, 0, 0)
            );
        }
    }
    #[test]
    fn one_food_type_and_housing_are_enough_to_grow_the_first_village() {
        let s = Satisfaction::calculate(NeedsInput {
            population: 7,
            capacity: 12,
            food: [30, 0, 0, 0],
            ..Default::default()
        });
        assert_eq!(s.happiness, 100);
        assert_eq!(s.immigration_seconds, 4);
        let hungry = Satisfaction::calculate(NeedsInput {
            population: 7,
            capacity: 12,
            ..Default::default()
        });
        assert!(hungry.food < 0 && hungry.happiness < s.happiness);
    }
    #[test]
    fn additional_needs_activate_at_their_population_thresholds() {
        for (threshold, factor) in [(8, 0), (16, 1), (16, 2), (24, 3), (32, 4), (40, 5)] {
            let score = |population| {
                Satisfaction::calculate(NeedsInput {
                    population,
                    capacity: 100,
                    food: [30; 4],
                    goods: [5; 3],
                    water: true,
                    healthcare: true,
                    leisure: 1,
                    employed: population,
                    workplaces: population,
                    ..Default::default()
                })
            };
            let get = |s: Satisfaction| {
                [
                    s.water,
                    s.variety,
                    s.employment,
                    s.health,
                    s.leisure,
                    s.goods,
                ][factor]
            };
            assert_eq!(get(score(threshold - 1)), 0);
            assert!(get(score(threshold)) > 0);
        }
    }
    #[test]
    fn mature_settlement_requires_balanced_supply_and_scores_are_bounded() {
        let input = NeedsInput {
            population: 40,
            capacity: 48,
            ..Default::default()
        };
        let hungry = Satisfaction::calculate(input);
        assert!(hungry.happiness < 40 && hungry.immigration_seconds == 0);
        let supplied = Satisfaction::calculate(NeedsInput {
            food: [u32::MAX; 4],
            goods: [5; 3],
            water: true,
            healthcare: true,
            leisure: 2,
            employed: 40,
            ..input
        });
        assert_eq!(supplied.happiness, 100);
        assert_eq!(supplied.immigration_seconds, 4);
    }
}
