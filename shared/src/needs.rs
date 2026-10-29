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
        let stock: u64 = input.food.into_iter().map(u64::from).sum();
        let food = if stock == 0 {
            if input.population == 0 {
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
        let variety = (types - 1).max(0) * 4;
        let housing = if input.population > input.capacity {
            -20
        } else if input.capacity.saturating_sub(input.population) >= 2 {
            10
        } else {
            5
        };
        let water = if input.water { 8 } else { -8 };
        let health = if input.healthcare { 5 } else { 0 };
        let leisure = input.leisure.min(2) as i32 * 5;
        let goods = input
            .goods
            .into_iter()
            .filter(|amount| *amount >= 5)
            .count() as i32
            * 2;
        let unemployed = input.population.saturating_sub(input.employed);
        let employment = if unemployed > 4 {
            -8
        } else if input.workplaces > input.employed {
            4
        } else if input.employed > 0 {
            3
        } else {
            0
        };
        let happiness =
            (50 + food + variety + housing + water + health + leisure + goods + employment)
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
#[cfg(test)]
mod tests {
    use super::*;
    fn settlement() -> NeedsInput {
        NeedsInput {
            population: 4,
            capacity: 8,
            ..Default::default()
        }
    }
    #[test]
    fn hunger_stops_immigration_and_food_diversity_improves_it() {
        let hungry = Satisfaction::calculate(settlement());
        assert!(hungry.happiness < 40 && hungry.immigration_seconds == 0);
        let single = Satisfaction::calculate(NeedsInput {
            food: [80, 0, 0, 0],
            capacity: 5,
            ..settlement()
        });
        let varied = Satisfaction::calculate(NeedsInput {
            food: [20; 4],
            capacity: 5,
            ..settlement()
        });
        assert!(single.happiness >= 50 && varied.happiness > single.happiness);
        assert!(varied.immigration_seconds < single.immigration_seconds);
    }
    #[test]
    fn housing_water_health_leisure_goods_and_employment_all_matter() {
        let base = NeedsInput {
            food: [20; 4],
            ..settlement()
        };
        let score = Satisfaction::calculate(base).happiness;
        assert!(
            Satisfaction::calculate(NeedsInput {
                capacity: 3,
                ..base
            })
            .happiness
                < score
        );
        for improved in [
            NeedsInput {
                water: true,
                ..base
            },
            NeedsInput {
                healthcare: true,
                ..base
            },
            NeedsInput { leisure: 1, ..base },
            NeedsInput {
                goods: [5; 3],
                ..base
            },
            NeedsInput {
                employed: 1,
                ..base
            },
        ] {
            assert!(Satisfaction::calculate(improved).happiness > score);
        }
        assert!(
            Satisfaction::calculate(NeedsInput {
                population: 8,
                ..base
            })
            .employment
                < 0
        );
    }
    #[test]
    fn scores_are_bounded_and_empty_settlement_can_attract_its_first_peasant() {
        let start = Satisfaction::calculate(NeedsInput {
            capacity: 4,
            ..Default::default()
        });
        assert_eq!(start.immigration_seconds, 14);
        let full = Satisfaction::calculate(NeedsInput {
            food: [u32::MAX; 4],
            goods: [u32::MAX; 3],
            water: true,
            healthcare: true,
            leisure: u32::MAX,
            ..settlement()
        });
        assert_eq!(full.happiness, 100);
        assert_eq!(full.immigration_seconds, 4);
    }
}
