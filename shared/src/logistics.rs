//! Physical storage and production rules shared by server and renderer.
use crate::{BuildingKind, Point};
pub const STORAGE_COLUMNS: usize = 6;
pub const STORAGE_ROWS: usize = 4;
pub const STORAGE_CELLS: usize = STORAGE_COLUMNS * STORAGE_ROWS;
pub const CARRY_CAPACITY: u32 = 5;
pub const PRODUCTION_SECONDS: u32 = 3;
pub const PRODUCTION_BUFFER: u32 = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Commodity {
    Wood = 0,
    Stone = 1,
    Iron = 2,
}
impl Commodity {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Wood),
            1 => Some(Self::Stone),
            2 => Some(Self::Iron),
            _ => None,
        }
    }
    pub fn cell_capacity(self) -> u32 {
        match self {
            Self::Wood => 20,
            Self::Stone => 54,
            Self::Iron => 96,
        }
    }
}
impl BuildingKind {
    pub fn produces(self) -> Option<Commodity> {
        match self {
            Self::SawPit => Some(Commodity::Wood),
            Self::Quarry => Some(Commodity::Stone),
            Self::IronMine => Some(Commodity::Iron),
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
    }
}
