//! One deterministic landscape shared by rendering, navigation and construction.
use crate::{Obstacle, Point};

pub const GRID: f32 = 2.0;
pub const STONE: [Point; 5] = [
    (-245.0, -140.0),
    (-110.0, -220.0),
    (310.0, -140.0),
    (-65.0, -35.0),
    (20.0, 75.0),
];
pub const IRON: [Point; 7] = [
    (-155.0, -110.0),
    (-285.0, -125.0),
    (230.0, -115.0),
    (275.0, 260.0),
    (-80.0, 15.0),
    (35.0, 65.0),
    (-20.0, -80.0),
];
pub const PEAKS: [(f32, f32, f32, f32); 7] = [
    (-275.0, -255.0, 78.0, 85.0),
    (-195.0, -275.0, 96.0, 76.0),
    (-185.0, -190.0, 58.0, 70.0),
    (285.0, -270.0, 68.0, 88.0),
    (230.0, -205.0, 44.0, 70.0),
    (-78.0, -74.0, 24.0, 25.0),
    (65.0, 100.0, 20.0, 25.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Grassland,
    Forest,
    Desert,
    Mountain,
    Iron,
    Stone,
    River,
    Ford,
}

impl Biome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grassland => "Grasland",
            Self::Forest => "Wald",
            Self::Desert => "Wüste",
            Self::Mountain => "Gebirge",
            Self::Iron => "Eisenvorkommen",
            Self::Stone => "Steinvorkommen",
            Self::River => "Fluss",
            Self::Ford => "Furt",
        }
    }
}

pub fn river_center(z: f32) -> f32 {
    92.0 + (z * 0.014).sin() * 36.0 + (z * 0.031).sin() * 9.0
}

pub fn river_distance(x: f32, z: f32) -> f32 {
    (x - river_center(z)).abs()
}

pub fn is_ford(z: f32) -> bool {
    [-185.0_f32, 35.0, 225.0]
        .iter()
        .any(|center| (z - center).abs() < 17.0)
}

fn mountain_height(x: f32, z: f32) -> f32 {
    PEAKS
        .iter()
        .map(|&(px, pz, height, radius)| {
            let d = ((x - px).powi(2) + (z - pz).powi(2)).sqrt() / radius;
            let ridge = (1.0 - d).max(0.0);
            height * ridge * ridge * (1.0 + 0.09 * (x * 0.17 + z * 0.11).sin())
        })
        .fold(0.0_f32, f32::max)
}

pub fn biome(x: f32, z: f32) -> Biome {
    if river_distance(x, z) < 12.0 {
        return if is_ford(z) {
            Biome::Ford
        } else {
            Biome::River
        };
    }
    if IRON.iter().any(|&(px, pz)| (x - px).hypot(z - pz) < 13.0) {
        return Biome::Iron;
    }
    if STONE.iter().any(|&(px, pz)| (x - px).hypot(z - pz) < 16.0) {
        return Biome::Stone;
    }
    if mountain_height(x, z) > 3.0 {
        return Biome::Mountain;
    }
    let desert_edge = 170.0 + 14.0 * (x * 0.021).sin();
    if (x < -45.0 && z > desert_edge)
        || (x > 175.0 && z > 240.0)
        || ((x + 25.0) / 28.0).powi(2) + ((z - 65.0) / 22.0).powi(2) < 1.0
    {
        return Biome::Desert;
    }
    let forest_west = ((x + 115.0) / 94.0).powi(2) + ((z - 55.0) / 116.0).powi(2);
    let forest_east = ((x - 278.0) / 104.0).powi(2) + ((z - 65.0) / 139.0).powi(2);
    let forest_start = ((x - 40.0) / 26.0).powi(2) + ((z + 43.0) / 24.0).powi(2);
    if x.hypot(z) > 42.0 && (forest_west < 1.0 || forest_east < 1.0 || forest_start < 1.0) {
        return Biome::Forest;
    }
    Biome::Grassland
}

fn raw_height(x: f32, z: f32) -> f32 {
    let mountain = mountain_height(x, z);
    if mountain > 0.0 {
        return mountain;
    }
    let distance = river_distance(x, z);
    if distance < 17.0 && !is_ford(z) {
        return -2.4 * (1.0 - distance / 17.0);
    }
    if biome(x, z) == Biome::Desert {
        let fade = ((z - 170.0) / 60.0).clamp(0.0, 1.0);
        return fade * (0.7 + 0.7 * (x * 0.035 + z * 0.07).sin());
    }
    0.0
}

/// Matches the triangles of the client mesh, including interpolation between vertices.
pub fn height(x: f32, z: f32) -> f32 {
    let x0 = (x / GRID).floor() * GRID;
    let z0 = (z / GRID).floor() * GRID;
    let tx = (x - x0) / GRID;
    let tz = (z - z0) / GRID;
    if tx + tz <= 1.0 {
        let a = raw_height(x0, z0);
        a + tx * (raw_height(x0 + GRID, z0) - a) + tz * (raw_height(x0, z0 + GRID) - a)
    } else {
        let d = raw_height(x0 + GRID, z0 + GRID);
        d + (1.0 - tx) * (raw_height(x0, z0 + GRID) - d)
            + (1.0 - tz) * (raw_height(x0 + GRID, z0) - d)
    }
}

pub fn color(x: f32, z: f32) -> [u8; 3] {
    let detail = ((x * 0.19).sin() * (z * 0.13).cos() * 5.0) as i16;
    let h = height(x, z);
    let base = match biome(x, z) {
        Biome::Grassland => [89, 141, 69],
        Biome::Forest => [47, 94, 47],
        Biome::Desert => [211, 181, 113],
        Biome::Iron => [148, 97, 62],
        Biome::Stone => [164, 166, 153],
        Biome::River => [40, 126, 169],
        Biome::Ford => [175, 160, 112],
        Biome::Mountain if h > 65.0 => [216, 222, 218],
        Biome::Mountain => [120, 123, 117],
    };
    base.map(|v| (v as i16 + detail).clamp(0, 255) as u8)
}

pub fn buildable(kind: crate::BuildingKind, x: f32, z: f32) -> bool {
    let terrain = biome(x, z);
    match kind {
        crate::BuildingKind::Fishpond => terrain == Biome::River,
        crate::BuildingKind::SawPit => terrain == Biome::Forest,
        crate::BuildingKind::Quarry => terrain == Biome::Stone,
        crate::BuildingKind::IronMine => terrain == Biome::Iron,
        _ => !matches!(
            terrain,
            Biome::River | Biome::Mountain | Biome::Stone | Biome::Iron
        ),
    }
}

/// A level foundation supports buildings even on gently undulating sand.
pub fn construction_height(kind: crate::BuildingKind, x: f32, z: f32, rotation: u8) -> Option<f32> {
    if kind == crate::BuildingKind::DrawbridgeGate {
        if (0..=23).any(|i| {
            let p = crate::fortifications::world((x, z), rotation, (0., -10. + i as f32 * 2.));
            matches!(
                biome(p.0, p.1),
                Biome::Mountain | Biome::Stone | Biome::Iron
            )
        }) {
            return None;
        }
        let bank = crate::fortifications::world((x, z), rotation, (0., 0.));
        let end = crate::fortifications::world((x, z), rotation, (0., 36.));
        if !matches!(
            biome(bank.0, bank.1),
            Biome::Mountain | Biome::Stone | Biome::Iron | Biome::River
        ) && !matches!(
            biome(end.0, end.1),
            Biome::Mountain | Biome::Stone | Biome::Iron | Biome::River
        ) {
            return Some(height(bank.0, bank.1).max(height(end.0, end.1)).max(0.));
        }
        return None;
    }

    if !x.is_finite() || !z.is_finite() {
        return None;
    }
    if kind == crate::BuildingKind::Fishpond {
        return (crate::ecology::fish_school((x, z)).is_some()
            && kind
                .rotated_build_samples(x, z, rotation)
                .all(|(x, z)| biome(x, z) == Biome::River))
        .then_some(-0.5);
    }
    let mut top = f32::NEG_INFINITY;
    for (px, pz) in kind.rotated_build_samples(x, z, rotation) {
        if !buildable(kind, px, pz) {
            return None;
        }
        top = top.max(height(px, pz));
    }
    Some(top)
}

/// Clear crowns as well as trunks within the rotated plot and model overhangs.
pub fn clearing_shape(kind: crate::BuildingKind, x: f32, z: f32, rotation: u8) -> Obstacle {
    let plot = kind.rotated_build_footprint(x, z, rotation);
    let model = kind.rotated_obstacle(x, z, rotation);
    Obstacle {
        x,
        z,
        half_x: plot.half_x.max(model.half_x),
        half_z: plot.half_z.max(model.half_z),
    }
}

/// A small number of conservative riverbank rectangles keeps navigation cheap.
/// Three dry crossings connect the two banks and remain free of these obstacles.
pub fn obstacles() -> Vec<Obstacle> {
    let mut result = Vec::new();
    let mut breaks: Vec<f32> = (0..=40).map(|i| -400.0 + i as f32 * 20.0).collect();
    for center in [-185.0_f32, 35.0, 225.0] {
        breaks.extend([center - 17.0, center + 17.0]);
    }
    breaks.sort_by(f32::total_cmp);
    for interval in breaks.windows(2) {
        let z = (interval[0] + interval[1]) * 0.5;
        if is_ford(z) {
            continue;
        }
        let x = river_center(z);
        let margin = (river_center(interval[0]) - x)
            .abs()
            .max((river_center(interval[1]) - x).abs());
        result.push(Obstacle {
            x,
            z,
            half_x: 13.0 + margin,
            half_z: (interval[1] - interval[0]) * 0.5,
        });
    }
    for &(x, z, _, _) in &PEAKS {
        result.push(Obstacle {
            x,
            z,
            half_x: 16.0,
            half_z: 16.0,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starting_area_has_nearby_varied_terrain_and_buildable_resources() {
        use crate::BuildingKind::*;
        for start in [(-32., 0.), (0., -36.), (0., 36.), (32., 0.)] {
            for target in [Biome::Forest, Biome::Desert, Biome::Mountain, Biome::River] {
                assert!(
                    (-140..=140).step_by(4).any(|x| (-140..=140)
                        .step_by(4)
                        .any(|z| crate::distance(start, (x as f32, z as f32)) < 150.
                            && biome(x as f32, z as f32) == target)),
                    "{start:?} lacks {target:?}"
                );
            }
            for (kind, points) in [(Quarry, STONE.as_slice()), (IronMine, IRON.as_slice())] {
                assert!(points.iter().any(|p| crate::distance(start, *p) < 120.
                    && construction_height(
                        kind,
                        kind.snap_position(p.0, p.1).0,
                        kind.snap_position(p.0, p.1).1,
                        0
                    )
                    .is_some()));
            }
        }
    }
    #[test]
    fn resource_buildings_require_their_entire_plot_on_the_right_resource() {
        use crate::BuildingKind::*;
        for (kind, point) in [
            (SawPit, (-115.0, 55.0)),
            (IronMine, IRON[0]),
            (Quarry, STONE[0]),
        ] {
            let (x, z) = kind.snap_position(point.0, point.1);
            assert!(construction_height(kind, x, z, 0).is_some());
            assert!(construction_height(kind, 25.0, 25.0, 0).is_none());
        }
        assert!(!buildable(SawPit, IRON[0].0, IRON[0].1));
        assert!(!buildable(Quarry, IRON[0].0, IRON[0].1));
        assert!(!buildable(IronMine, STONE[0].0, STONE[0].1));
        assert!(construction_height(IronMine, IRON[0].0 + 12.0, IRON[0].1, 0).is_none());
    }
    #[test]
    fn ordinary_buildings_allow_forest_and_dunes_but_not_rock_or_water() {
        use crate::BuildingKind::*;
        for kind in [Granary, Housing, Barracks, Stockpile] {
            for (x, z) in [(25.0, 25.0), (-115.0, 55.0), (-200.0, 300.0)] {
                let (x, z) = kind.snap_position(x, z);
                let top = construction_height(kind, x, z, 0)
                    .expect("normal land must allow construction");
                assert!(kind
                    .build_samples(x, z)
                    .all(|(px, pz)| height(px, pz) <= top + 0.0001));
            }
            for (x, z) in [
                STONE[0],
                IRON[0],
                (-195.0, -275.0),
                (river_center(0.0), 0.0),
            ] {
                assert!(construction_height(kind, x, z, 0).is_none());
            }
        }
    }
    #[test]
    fn tree_clearance_covers_the_rotated_model_and_its_crowns() {
        let kind = crate::BuildingKind::TowerStairwell;
        let first = clearing_shape(kind, -115.0, 55.0, 0);
        let turned = clearing_shape(kind, -115.0, 55.0, 1);
        assert_eq!(first.half_x, turned.half_z);
        assert_eq!(first.half_z, turned.half_x);
        assert!(turned.contains((-115.0 + turned.half_x + 1.0, 55.0), 2.5));
        assert!(!turned.contains((-115.0 + turned.half_x + 5.0, 55.0), 2.5));
    }
    #[test]
    fn landscape_contains_all_requested_types() {
        assert_eq!(biome(0.0, 0.0), Biome::Grassland);
        assert_eq!(biome(-115.0, 55.0), Biome::Forest);
        assert_eq!(biome(-200.0, 300.0), Biome::Desert);
        assert_eq!(biome(-195.0, -275.0), Biome::Mountain);
        assert_eq!(biome(IRON[0].0, IRON[0].1), Biome::Iron);
        assert_eq!(biome(river_center(0.0), 0.0), Biome::River);
        assert!(height(-195.0, -275.0) > 70.0);
        assert!(height(river_center(0.0), 0.0) < -1.0);
    }
    #[test]
    fn rivers_have_reachable_dry_crossings_and_banks_are_blocked() {
        let blocked = obstacles();
        assert!(!crate::free((river_center(0.0), 0.0), &blocked));
        for z in [-185.0, 35.0, 225.0] {
            let x = river_center(z);
            assert_eq!(biome(x, z), Biome::Ford);
            assert_eq!(height(x, z), 0.0);
            assert!(crate::segment_clear((x - 32.0, z), (x + 32.0, z), &blocked));
        }
        for z in (-399..400).step_by(2) {
            let z = z as f32;
            if !is_ford(z) {
                assert!(
                    !crate::free((river_center(z), z), &blocked),
                    "river gap at {z}"
                );
            }
        }
    }
    #[test]
    fn navigation_routes_between_banks_via_a_crossing() {
        let blocked = obstacles();
        let start = (-10.0, 0.0);
        let goal = (210.0, 0.0);
        let path = crate::shortest_path(start, goal, &blocked).expect("banks must connect");
        assert!(path.len() > 1);
        let mut previous = start;
        for next in path {
            assert!(crate::segment_clear(previous, next, &blocked));
            previous = next;
        }
        assert_eq!(previous, goal);
    }
    #[test]
    fn spawn_and_mountain_heights_are_finite_and_construction_is_checked() {
        assert_eq!(height(0.0, 0.0), 0.0);
        for x in (-400..=400).step_by(8) {
            for z in (-400..=400).step_by(8) {
                assert!(height(x as f32, z as f32).is_finite());
            }
        }
        assert!(!buildable(
            crate::BuildingKind::Granary,
            river_center(0.0),
            0.0
        ));
        assert!(buildable(crate::BuildingKind::Granary, -115.0, 55.0));
        assert!(buildable(
            crate::BuildingKind::IronMine,
            IRON[0].0,
            IRON[0].1
        ));
        for (x, z) in IRON {
            let mine = crate::BuildingKind::IronMine;
            let (x, z) = mine.snap_position(x, z);
            assert!(mine
                .build_samples(x, z)
                .all(|(px, pz)| buildable(mine, px, pz)
                    && (height(px, pz) - height(x, z)).abs() <= 0.05));
        }
    }
}
