//! Settlement policy and public market rules. Gold is counted in whole sesterces.
pub const RATION_LABELS: [&str; 8] = [
    "Hungern",
    "Winzige Rationen",
    "Knappe Rationen",
    "Reduzierte Rationen",
    "Normale Rationen",
    "Großzügige Rationen",
    "Üppige Rationen",
    "Festmahl",
];
pub const RATION_QUARTERS: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 8];
pub const RATION_HAPPINESS: [i32; 8] = [-35, -25, -14, -6, 0, 5, 10, 18];
pub const TAX_LABELS: [&str; 8] = [
    "Steuerfrei",
    "Sehr niedrig",
    "Niedrig",
    "Normal",
    "Erhöht",
    "Hoch",
    "Sehr hoch",
    "Extrem",
];
pub const TAX_RATES: [u64; 8] = [0, 1, 2, 3, 4, 6, 8, 12];
pub const TAX_HAPPINESS: [i32; 8] = [12, 8, 4, 0, -6, -12, -20, -30];
pub const GOODS: [&str; 8] = [
    "Holz", "Stein", "Eisen", "Äpfel", "Brot", "Käse", "Fleisch", "Fisch",
];
pub const BASE_PRICES: [i64; 8] = [3, 4, 6, 2, 3, 4, 5, 3];
pub const STARTING_GOLD: u64 = 50;
pub fn ration_use(population: u32, level: u8, remainder: u32) -> (u32, u32) {
    let quarters = population
        .saturating_mul(RATION_QUARTERS[level.min(7) as usize])
        .saturating_add(remainder);
    (quarters / 16, quarters % 16)
}
pub fn immigration(happiness: u32) -> u32 {
    match happiness {
        80..=100 => 4,
        65..=79 => 8,
        50..=64 => 14,
        _ => 0,
    }
}
pub fn quote(resource: u8, pressure: i32, amount: u32, buy: bool) -> u64 {
    if resource > 7 || amount > 200 {
        return u64::MAX;
    }
    (0..amount)
        .map(|i| {
            let p = (pressure + if buy { i as i32 } else { -(i as i32) }).clamp(-60, 300) as i64;
            let value = BASE_PRICES[resource as usize] * (100 + p);
            if buy {
                ((value * 115 + 9999) / 10000).max(1) as u64
            } else {
                ((value * 85) / 10000).max(1) as u64
            }
        })
        .sum()
}
/// Barter uses the same marginal prices as a sale followed by a purchase.
pub fn barter(sell: u8, sell_pressure: i32, amount: u32, buy: u8, buy_pressure: i32) -> (u32, u64) {
    if sell > 7 || buy > 7 || sell == buy || amount == 0 || amount > 200 {
        return (0, 0);
    }
    let budget = quote(sell, sell_pressure, amount, false);
    let received = (1..=200)
        .take_while(|n| quote(buy, buy_pressure, *n, true) <= budget)
        .last()
        .unwrap_or(0);
    (received, budget - quote(buy, buy_pressure, received, true))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn barter_spends_only_sale_value_and_returns_exact_change() {
        for sell in 0..8 {
            for buy in 0..8 {
                if sell == buy {
                    assert_eq!(barter(sell, 0, 10, buy, 0), (0, 0));
                    continue;
                }
                let (n, change) = barter(sell, 0, 10, buy, 0);
                assert_eq!(quote(buy, 0, n, true) + change, quote(sell, 0, 10, false));
                assert!(n == 200 || quote(buy, 0, n + 1, true) > quote(sell, 0, 10, false));
            }
        }
    }
    #[test]
    fn exact_rations_carry_fractional_meals_instead_of_rounding_up() {
        let (mut sum, mut carry) = (0, 0);
        for _ in 0..16 {
            let (n, c) = ration_use(1, 4, carry);
            sum += n;
            carry = c;
        }
        assert_eq!(sum, 4);
        assert_eq!(ration_use(8, 0, 0), (0, 0));
        assert_eq!(ration_use(8, 7, 0), (4, 0));
    }
    #[test]
    fn demand_changes_prices_without_round_trip_profit() {
        for r in 0..8 {
            for p in [-60, 0, 150, 300] {
                for n in [1, 5, 20, 100] {
                    let buy = quote(r, p, n, true);
                    let sell = quote(r, (p + n as i32).clamp(-60, 300), n, false);
                    assert!(sell <= buy);
                    assert!(quote(r, p + 20, n, true) >= buy);
                }
            }
        }
    }
}
