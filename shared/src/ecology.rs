//! Deterministic sites and bounded ecology/transport rules shared by client and server.
use crate::{terrain, Point};
pub const CART_CAPACITY: u32 = 20;
pub const TREE_WOOD: u32 = 24;
pub const STONE_AMOUNT: u32 = 18000;
pub const IRON_AMOUNT: u32 = 14000;
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
    (3, 2, -320., 180.),
    (3, 0, 300., -300.),
    (3, 3, 315., 300.),
    (4, 1, -320., -320.),
    (4, 1, -320., 315.),
    (4, 0, 320., -300.),
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
        assert!(HERDS.iter().any(|h| h.0 == 3 && h.1 >= 3));
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

pub const PREDATOR_START_DISTANCE: f32 = 160.;
pub const CART_RANGE: f32 = 80.;
pub fn production_seconds(kind: &str) -> u32 {
    if kind == "wheat_farm" {
        return 6;
    }
    if kind == "windmill" || kind == "bakery" {
        return 4;
    }

    match kind {
        "saw_pit" => 6,
        "quarry" => 6,
        "iron_mine" => 7,
        _ => crate::logistics::PRODUCTION_SECONDS,
    }
}
pub fn deposit_capacity(kind: u8, index: usize) -> u32 {
    let (sites, radii, biome, density) = if kind == STONE {
        (
            &terrain::STONE[..],
            &terrain::STONE_RADII[..],
            terrain::Biome::Stone,
            18,
        )
    } else {
        (
            &terrain::IRON[..],
            &terrain::IRON_RADII[..],
            terrain::Biome::Iron,
            26,
        )
    };
    let centre = sites[index];
    let r = radii[index].ceil() as i32;
    let mut cells = 0;
    for x in (-r..=r).step_by(2) {
        for z in (-r..=r).step_by(2) {
            let p = (centre.0 + x as f32, centre.1 + z as f32);
            if crate::distance(p, centre) < radii[index] && terrain::biome(p.0, p.1) == biome {
                cells += 1;
            }
        }
    }
    cells * 4 * density
}
pub fn raid_attempt(game: u64, second: u32, group: u32) -> bool {
    let mut x = game.wrapping_mul(0x9e3779b97f4a7c15)
        ^ (second as u64).wrapping_mul(0xbf58476d1ce4e5b9)
        ^ (group as u64).wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x % 1800 == 0
}

pub fn defence(job: &str) -> (f32, f32) {
    match job {
        "hunters_post" => (14., 0.7),
        "quarry" | "iron_mine" => (2.1, 0.65),
        "saw_pit" => (1.9, 0.45),
        "fishpond" => (1.9, 0.3),
        _ => (1.7, 0.2),
    }
}

#[cfg(test)]
mod balance_tests {
    use super::*;
    #[test]
    fn predators_are_sparse_remote_but_raids_have_no_minimum_age() {
        assert_eq!(
            HERDS
                .iter()
                .filter(|h| predator(h.0))
                .map(|h| h.1 as u32)
                .sum::<u32>(),
            7
        );
        for h in HERDS.iter().filter(|h| predator(h.0) && h.1 > 0) {
            assert!((h.2 * h.2 + h.3 * h.3).sqrt() > PREDATOR_START_DISTANCE);
        }
        assert!((1..12000).any(|game| raid_attempt(game, 1, 9)));
        assert!((1..1200).any(|game| !raid_attempt(game, 1, 9)));
    }
    #[test]
    fn professions_defend_with_different_weapons() {
        assert!(defence("hunters_post").0 > defence("quarry").0);
        assert!(defence("hunters_post").1 > defence("quarry").1);
        assert!(defence("quarry").1 > defence("fishpond").1);
    }
    #[test]
    fn fishery_fits_river_and_rejects_land() {
        let k = crate::BuildingKind::Fishpond;
        assert!(k.grid_size().0 <= 3);
        let mut n = 0;
        for z in -60..0 {
            for x in 40..90 {
                if terrain::construction_height(k, x as f32, z as f32, 0).is_some() {
                    n += 1;
                    assert_eq!(terrain::biome(x as f32, z as f32), terrain::Biome::River);
                }
            }
        }
        assert!(n > 0);
        assert!(terrain::construction_height(k, 0., 0., 0).is_none());
    }
}

#[cfg(test)]
mod extraction_tests {
    use super::*;
    #[test]
    fn deposits_scale_with_actual_field_area_and_take_hours_to_exhaust() {
        let small = deposit_capacity(STONE, 3);
        let large = deposit_capacity(STONE, 2);
        assert!(small > 10000);
        assert!(large > small * 2);
        assert!(deposit_capacity(IRON, 3) > deposit_capacity(IRON, 4));
        assert!(TREE_WOOD * production_seconds("saw_pit") >= 120);
        assert_eq!(crate::STARTING_RESOURCE_AMOUNT, 120);
    }
    #[test]
    fn cart_can_stand_on_both_resource_fields() {
        for k in [crate::BuildingKind::OxTether] {
            assert!(terrain::construction_height(k, -55., -35., 0).is_some());
            assert!(terrain::construction_height(k, -77., 15., 0).is_some());
        }
    }
}
