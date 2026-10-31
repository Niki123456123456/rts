//! Physical storage and production rules shared by server and renderer.
use crate::{BuildingKind, Point};
pub const STORAGE_COLUMNS: usize = 6;
pub const STORAGE_ROWS: usize = 4;
pub const STORAGE_CELLS: usize = STORAGE_COLUMNS * STORAGE_ROWS;
pub const CARRY_CAPACITY: u32 = 5;
pub const PRODUCTION_SECONDS: u32 = 3;
pub const PRODUCTION_BUFFER: u32 = 20;
pub const STARTING_FOOD_AMOUNT: u32 = 5;
pub const FOOD_CAPACITY_PER_KIND: u32 = 24;

#[derive(Clone, Copy, Debug)]
pub struct FoodVisual {
    pub model: usize,
    pub position: Point,
    pub layer: u32,
}

/// Four separated stacks per food type along the inner walls. Leave the
/// middle of the front edge clear for the granary entrance.
pub fn food_visuals(stock: [u32; 4]) -> Vec<FoodVisual> {
    let mut result = Vec::new();
    for (resource, amount) in stock.into_iter().enumerate() {
        for unit in 0..amount {
            let slot = (unit % 4) as usize;
            let along = [-1.2, -0.4, 0.4, 1.2][slot];
            let position = match resource {
                0 => (along, -2.0),
                1 => (-2.0, along),
                2 => (2.0, along),
                _ => ([-1.8, -1.1, 1.1, 1.8][slot], 2.0),
            };
            result.push(FoodVisual {
                model: 22 + resource,
                position,
                layer: unit / 4,
            });
        }
    }
    result
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Commodity {
    Wood = 0,
    Stone = 1,
    Iron = 2,
    Apples = 3,
    Bread = 4,
    Cheese = 5,
    Meat = 6,
    Fish = 7,
    Wheat = 8,
    Flour = 9,
    Tools = 11,
    Grapes = 12,
    Olives = 13,
    Wine = 14,
    Oil = 15,
}
impl Commodity {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Wood),
            1 => Some(Self::Stone),
            2 => Some(Self::Iron),
            3 => Some(Self::Apples),
            4 => Some(Self::Bread),
            5 => Some(Self::Cheese),
            6 => Some(Self::Meat),
            7 => Some(Self::Fish),
            8 => Some(Self::Wheat),
            9 => Some(Self::Flour),
            11 => Some(Self::Tools),
            12 => Some(Self::Grapes),
            13 => Some(Self::Olives),
            14 => Some(Self::Wine),
            15 => Some(Self::Oil),
            _ => None,
        }
    }
    pub fn cell_capacity(self) -> u32 {
        match self {
            Self::Wood => 20,
            Self::Stone => 54,
            Self::Iron => 96,
            Self::Apples | Self::Bread | Self::Cheese | Self::Meat | Self::Fish => {
                FOOD_CAPACITY_PER_KIND
            }
            Self::Wheat
            | Self::Flour
            | Self::Tools
            | Self::Grapes
            | Self::Olives
            | Self::Wine
            | Self::Oil => 120,
        }
    }
    pub fn is_food(self) -> bool {
        matches!(
            self,
            Self::Apples | Self::Bread | Self::Cheese | Self::Meat | Self::Fish
        )
    }
}
impl BuildingKind {
    pub fn produces(self) -> Option<Commodity> {
        match self {
            Self::SawPit
            | Self::Stable
            | Self::SiegeWorkshop
            | Self::ResearchCenter
            | Self::WeaponSmith
            | Self::ToolSmith
            | Self::Hospital => Some(Commodity::Wood),
            Self::Vineyard => Some(Commodity::Grapes),
            Self::OliveGrove => Some(Commodity::Olives),
            Self::VintnersWorkshop => Some(Commodity::Wine),
            Self::OlivePress => Some(Commodity::Oil),
            Self::WheatFarm => Some(Commodity::Wheat),
            Self::Windmill => Some(Commodity::Flour),
            Self::Bakery => Some(Commodity::Bread),
            Self::Quarry => Some(Commodity::Stone),
            Self::IronMine => Some(Commodity::Iron),
            Self::DairyFarm => Some(Commodity::Cheese),
            Self::AppleFarm => Some(Commodity::Apples),
            Self::Fishpond => Some(Commodity::Fish),
            Self::HuntersPost => Some(Commodity::Meat),
            _ => None,
        }
    }
}
pub fn storage_cells(stock: [u32; 3]) -> u64 {
    [Commodity::Wood, Commodity::Stone, Commodity::Iron]
        .into_iter()
        .zip(stock)
        .map(|(r, n)| u64::from(n.div_ceil(r.cell_capacity())))
        .sum()
}
pub fn can_store(mut stock: [u32; 3], resource: Commodity, amount: u32) -> bool {
    if resource.is_food() || resource.is_intermediate() {
        return false;
    }
    let Some(sum) = stock[resource as usize].checked_add(amount) else {
        return false;
    };
    stock[resource as usize] = sum;
    storage_cells(stock) <= STORAGE_CELLS as u64
}
#[derive(Clone, Copy, Debug)]
pub struct StockVisual {
    pub model: usize,
    pub cell: usize,
    pub height: f32,
    pub quantity: u32,
}
pub fn cell_position(cell: usize) -> Point {
    (
        (cell % STORAGE_COLUMNS) as f32 * 2. - 5.,
        (cell / STORAGE_COLUMNS) as f32 * 2. - 3.,
    )
}
/// 0 wood bundle; 1 stone layer; 2..9 single stones; 10 iron layer; 11..21 partial iron layers.
pub fn stock_visuals(stock: [u32; 3]) -> Vec<StockVisual> {
    assert!(storage_cells(stock) <= STORAGE_CELLS as u64);
    let mut result = Vec::new();
    let mut cell = 0;
    for (resource, mut amount) in [Commodity::Wood, Commodity::Stone, Commodity::Iron]
        .into_iter()
        .zip(stock)
    {
        while amount > 0 {
            let count = amount.min(resource.cell_capacity());
            match resource {
                Commodity::Wood => {
                    for layer in 0..count {
                        result.push(StockVisual {
                            model: 0,
                            cell,
                            height: layer as f32 * 0.125,
                            quantity: 1,
                        });
                    }
                }
                Commodity::Stone => {
                    for layer in 0..count / 9 {
                        result.push(StockVisual {
                            model: 1,
                            cell,
                            height: layer as f32 * 0.325,
                            quantity: 9,
                        });
                    }
                    for part in 0..count % 9 {
                        result.push(StockVisual {
                            model: 2 + part as usize,
                            cell,
                            height: (count / 9) as f32 * 0.325,
                            quantity: 1,
                        });
                    }
                }
                Commodity::Iron => {
                    for layer in 0..count / 12 {
                        result.push(StockVisual {
                            model: 10,
                            cell,
                            height: layer as f32 * 0.21,
                            quantity: 12,
                        });
                    }
                    if count % 12 > 0 {
                        result.push(StockVisual {
                            model: 10 + (count % 12) as usize,
                            cell,
                            height: (count / 12) as f32 * 0.21,
                            quantity: count % 12,
                        });
                    }
                }
                _ => {
                    unreachable!("Food is stored in granaries")
                }
            }
            amount -= count;
            cell += 1;
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn food_stacks_show_exact_counts_along_walls_and_keep_the_entry_clear() {
        for amount in 0..=40 {
            let visuals = food_visuals([amount; 4]);
            assert_eq!(visuals.len(), amount as usize * 4);
            for resource in 0..4 {
                assert_eq!(
                    visuals.iter().filter(|v| v.model == 22 + resource).count(),
                    amount as usize
                );
            }
            for p in visuals {
                assert!(p.position.0.abs() <= 2. && p.position.1.abs() <= 2.);
                assert!(p.position.0.abs() == 2. || p.position.1.abs() == 2.);
                assert!(!(p.position.1 == 2. && p.position.0.abs() < 1.));
            }
        }
    }
    #[test]
    fn every_quantity_is_represented_exactly_and_inside_the_grid() {
        for resource in 0..3 {
            for amount in 0..=400 {
                let mut stock = [0; 3];
                stock[resource] = amount;
                let visuals = stock_visuals(stock);
                assert_eq!(visuals.iter().map(|p| p.quantity).sum::<u32>(), amount);
                assert!(visuals
                    .iter()
                    .all(|p| p.cell < STORAGE_CELLS && p.model < 22 && p.height < 2.6));
            }
        }
        let visuals = stock_visuals([20; 3]);
        assert_eq!(visuals.iter().map(|p| p.quantity).sum::<u32>(), 60);
        assert_eq!(storage_cells([20; 3]), 3);
        assert!(!can_store([480, 0, 0], Commodity::Stone, 1));
        assert!(!can_store([u32::MAX, 0, 0], Commodity::Wood, 1));
        assert!(!can_store([0; 3], Commodity::Cheese, 1));
    }
}

impl Commodity {
    pub fn is_intermediate(self) -> bool {
        matches!(
            self,
            Self::Wheat
                | Self::Flour
                | Self::Tools
                | Self::Grapes
                | Self::Olives
                | Self::Wine
                | Self::Oil
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Wood => "Holz",
            Self::Stone => "Stein",
            Self::Iron => "Eisen",
            Self::Apples => "Äpfel",
            Self::Bread => "Brot",
            Self::Cheese => "Käse",
            Self::Meat => "Fleisch",
            Self::Fish => "Fisch",
            Self::Wheat => "Korn",
            Self::Flour => "Mehl",
            Self::Tools => "Werkzeuge",
            Self::Grapes => "Trauben",
            Self::Olives => "Oliven",
            Self::Wine => "Wein",
            Self::Oil => "Olivenöl",
        }
    }
}
