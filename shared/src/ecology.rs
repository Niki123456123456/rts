//! Deterministic sites and bounded ecology/transport rules shared by client and server.
use crate::{terrain, Point};
pub const CART_CAPACITY: u32 = 20;
pub const TREE_WOOD: u32 = 6;
pub const STONE_AMOUNT: u32 = 180;
pub const IRON_AMOUNT: u32 = 140;
pub const FISH_CAPACITY: u32 = 80;
pub const FISH_REGEN_SECONDS: u32 = 12;
pub const FISH_Z: [f32; 8] = [-320., -230., -120., -40., 75., 145., 280., 345.];
pub const TREE: u8 = 0;
pub const STONE: u8 = 1;
pub const IRON: u8 = 2;
pub const FISH: u8 = 7;
#[derive(Clone, Copy)]
pub struct TreeSite {
    pub site: u32,
    pub x: f32,
    pub z: f32,
    pub scale: f32,
    pub evergreen: bool,
}
pub fn trees() -> Vec<TreeSite> {
    let mut result = Vec::new();
    for row in 0..88u32 {
        for column in 0..88u32 {
            let hash = column.wrapping_mul(73856093) ^ row.wrapping_mul(19349663);
            let x = -392. + column as f32 * 9. + (hash % 51) as f32 * 0.1;
            let z = -392. + row as f32 * 9. + ((hash >> 8) % 51) as f32 * 0.1;
            if terrain::biome(x, z) == terrain::Biome::Forest && hash % 5 != 0 {
                result.push(TreeSite {
                    site: row * 88 + column + 1,
                    x,
                    z,
                    scale: 0.8 + (hash % 50) as f32 * 0.012,
                    evergreen: hash % 3 == 0,
                });
            }
        }
    }
    result
}
pub fn fish_center(index: usize) -> Point {
    let z = FISH_Z[index];
    (terrain::river_center(z), z)
}
pub fn fish_school(point: Point) -> Option<usize> {
    (0..FISH_Z.len())
        .filter(|i| crate::distance(point, fish_center(*i)) <= 34.)
        .min_by(|a, b| {
            crate::distance(point, fish_center(*a))
                .total_cmp(&crate::distance(point, fish_center(*b)))
        })
}
pub fn requires_cart(resource: u8) -> bool {
    resource == STONE || resource == IRON
}
pub fn carry_capacity(cart: bool) -> u32 {
    if cart {
        CART_CAPACITY
    } else {
        crate::logistics::CARRY_CAPACITY
    }
}
pub fn load_ready(resource: u8, amount: u32, cart: bool, exhausted: bool) -> bool {
    amount > 0
        && (!requires_cart(resource) || cart)
        && ((resource == 3 || resource == 5) || amount >= carry_capacity(cart) || exhausted)
}
pub fn consume(remaining: u32, amount: u32) -> (u32, u32) {
    let take = remaining.min(amount);
    (remaining - take, take)
}
pub fn regenerate_fish(remaining: u32, tick: u32) -> u32 {
    if tick % FISH_REGEN_SECONDS == 0 {
        remaining.saturating_add(1).min(FISH_CAPACITY)
    } else {
        remaining
    }
}
pub fn species_name(species: u8) -> &'static str {
    match species {
        0 => "Reh",
        1 => "Hase",
        2 => "Wildschwein",
        3 => "Wolf",
        _ => "Bär",
    }
}
pub fn predator(species: u8) -> bool {
    species >= 3
}
pub fn animal_health(species: u8) -> f32 {
    match species {
        0 => 1.,
        1 => 0.45,
        2 => 1.5,
        3 => 1.4,
        _ => 3.5,
    }
}
pub fn animal_meat(species: u8) -> u32 {
    match species {
        0 => 10,
        1 => 3,
        2 => 15,
        3 => 8,
        _ => 24,
    }
}
pub fn aggression_radius(species: u8) -> f32 {
    match species {
        3 => 9.,
        4 => 11.,
        _ => 0.,
    }
}
pub const HERDS: [(u8, u32, f32, f32); 15] = [
    (0, 4, -55., 55.),
    (0, 7, -135., 75.),
    (0, 3, 48., -60.),
    (0, 5, 235., 90.),
    (1, 5, -40., 85.),
    (1, 10, 25., -100.),
    (1, 4, 210., 180.),
    (2, 3, -100., -45.),
    (2, 5, 200., 45.),
    (3, 3, -120., 110.),
    (3, 6, 180., -90.),
    (3, 9, -230., -100.),
    (4, 1, -100., -130.),
    (4, 2, 265., 125.),
    (4, 1, -230., 240.),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carts_are_required_for_stone_and_metal_and_quadruple_wood_loads() {
        assert!(!load_ready(STONE, 20, false, false));
        assert!(!load_ready(IRON, 20, false, true));
        assert!(load_ready(STONE, 20, true, false));
        assert_eq!(carry_capacity(true), 4 * carry_capacity(false));
        assert!(!load_ready(TREE, 5, true, false));
        assert!(load_ready(TREE, 20, true, false));
        assert!(load_ready(TREE, 3, true, true));
    }
    #[test]
    fn resources_cannot_be_created_by_over_harvesting() {
        let (mut stock, mut sum) = (6, 0);
        for _ in 0..10 {
            let (n, t) = consume(stock, 1);
            stock = n;
            sum += t;
        }
        assert_eq!((stock, sum), (0, 6));
    }
    #[test]
    fn overfished_schools_recover_slowly_and_never_exceed_capacity() {
        let mut stock = 0;
        for t in 1..=24 {
            stock = regenerate_fish(stock, t);
        }
        assert_eq!(stock, 2);
        assert_eq!(regenerate_fish(FISH_CAPACITY, 12), FISH_CAPACITY);
    }
    #[test]
    fn fishing_is_limited_to_discrete_sections_of_the_river() {
        let c = fish_center(3);
        assert!(fish_school((c.0 - 24., c.1)).is_some());
        assert!(fish_school((0., 0.)).is_none());
        assert!(fish_school((terrain::river_center(10.), 10.)).is_none());
    }
    #[test]
    fn wildlife_has_different_group_sizes_and_predators() {
        assert!(HERDS.iter().any(|h| h.0 == 3 && h.1 >= 6));
        assert!(HERDS.iter().any(|h| h.0 == 4 && h.1 == 1));
        assert_eq!(aggression_radius(0), 0.);
        assert!(aggression_radius(4) > aggression_radius(3));
    }
    #[test]
    fn tree_sites_are_stable_unique_and_really_in_forests() {
        let sites = trees();
        assert!(sites.len() > 300);
        let mut ids = std::collections::HashSet::new();
        for s in sites {
            assert!(ids.insert(s.site));
            assert_eq!(terrain::biome(s.x, s.z), terrain::Biome::Forest);
        }
    }
}
