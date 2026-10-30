//! Shared building catalog, collision footprints and continuous shortest-path navigation.
//! Bounds come from POSITION accessor 0 of each collision GLB, scaled 1:100.
pub const MODEL_SCALE: f32 = 0.01;
pub const MODEL_CENTER: (f32, f32) = (308.07544, -313.6961);
pub const HALF_X: f32 = (581.97156 - 34.17932) * MODEL_SCALE / 2.0;
pub const HALF_Z: f32 = (567.28033 - 60.11186) * MODEL_SCALE / 2.0;
mod building_catalog;
pub mod building_roof;
pub mod camp;
pub mod civic;
pub mod ecology;
pub mod fortifications;
pub mod logistics;
pub mod needs;
pub mod placement;
pub mod population;
pub mod terrain;
pub mod walls;
pub use building_catalog::BuildingKind;

pub const MAP_SIZE: f32 = 800.0;
pub const MAP_LIMIT: f32 = MAP_SIZE * 0.5 - 1.0;
/// Grid lines lie at multiples of two metres on both world axes.
pub const BUILD_GRID_SIZE: f32 = 2.0;
pub const STARTING_RESOURCE_AMOUNT: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
}
impl Cost {
    pub const fn new(wood: u32, stone: u32, iron: u32) -> Self {
        Self { wood, stone, iron }
    }
    pub fn affordable(self, wood: u32, stone: u32, iron: u32) -> bool {
        wood >= self.wood && stone >= self.stone && iron >= self.iron
    }
    pub fn label(self) -> String {
        format!(
            "{} Holz · {} Stein · {} Eisen",
            self.wood, self.stone, self.iron
        )
    }
}
impl BuildingKind {
    pub fn cost(self) -> Cost {
        use BuildingKind::*;
        let (w, s, i) = match self {
            WoodenWall => (2, 0, 0),
            StoneWallLow => (0, 1, 0),
            StoneWall => (0, 2, 0),
            StoneWallHigh => (0, 3, 0),
            StoneWallCrenellated => (0, 3, 0),
            GatehouseWooden => (14, 0, 2),
            GatehouseSmall => (8, 18, 3),
            GatehouseMedium => (12, 28, 4),
            GatehouseLarge => (16, 40, 6),
            DrawbridgeGate => (22, 30, 8),
            Keep => (40, 60, 10),
            Barracks => (15, 20, 5),
            Stockpile => (4, 0, 0),
            Granary => (8, 3, 0),
            TowerWooden | Watchpost => (12, 0, 0),
            TowerLookout | TowerStairwell => (5, 12, 0),
            TowerSquare | TowerRoundShort | TowerRoundTall | TowerGreat => (8, 24, 2),
            Quarry | IronMine => (12, 4, 2),
            OxTether => (8, 2, 2),
            SawPit => (6, 0, 0),
            HuntersPost => (8, 0, 2),
            Fishpond => (10, 0, 1),
            Housing => (6, 2, 0),
            Well => (4, 5, 0),
            Inn | Apothecary => (12, 6, 0),
            _ => (8, 3, 0),
        };
        Cost::new(w, s, i)
    }
    pub fn stone_cost(self) -> u32 {
        self.cost().stone
    }
    pub fn is_wall(self) -> bool {
        matches!(
            self,
            Self::StoneWall
                | Self::StoneWallLow
                | Self::StoneWallHigh
                | Self::StoneWallCrenellated
                | Self::WoodenWall
        )
    }
    pub fn is_gate(self) -> bool {
        matches!(
            self,
            Self::GatehouseWooden
                | Self::GatehouseSmall
                | Self::GatehouseMedium
                | Self::GatehouseLarge
                | Self::DrawbridgeGate
        )
    }
    /// Reserve visible bounds, including stairs, without mandatory spacer plots.
    /// Wall sections share their small mortar projections.
    pub fn construction_bounds(self, x: f32, z: f32, rotation: u8) -> Obstacle {
        if self.is_wall() {
            self.rotated_build_footprint(x, z, rotation)
        } else {
            self.rotated_obstacle(x, z, rotation)
        }
    }
}

impl BuildingKind {
    pub fn rotated_grid_size(self, rotation: u8) -> (u16, u16) {
        let (width, depth) = self.grid_size();
        if rotation % 2 == 0 {
            (width, depth)
        } else {
            (depth, width)
        }
    }

    pub fn snap_position(self, x: f32, z: f32) -> Point {
        self.rotated_snap_position(x, z, 0)
    }

    /// Odd cell counts have odd-metre centres; dimensions follow the orientation.
    pub fn rotated_snap_position(self, x: f32, z: f32, rotation: u8) -> Point {
        let (width, depth) = self.rotated_grid_size(rotation);
        let snap = |value: f32, cells: u16| {
            let offset = (cells % 2) as f32 * BUILD_GRID_SIZE * 0.5;
            ((value - offset) / BUILD_GRID_SIZE).round() * BUILD_GRID_SIZE + offset
        };
        (snap(x, width), snap(z, depth))
    }

    pub fn build_footprint(self, x: f32, z: f32) -> Obstacle {
        self.rotated_build_footprint(x, z, 0)
    }

    /// Logical construction area; orientation swaps its world-space dimensions.
    pub fn rotated_build_footprint(self, x: f32, z: f32, rotation: u8) -> Obstacle {
        let (width, depth) = self.rotated_grid_size(rotation);
        Obstacle {
            x,
            z,
            half_x: width as f32 * BUILD_GRID_SIZE * 0.5,
            half_z: depth as f32 * BUILD_GRID_SIZE * 0.5,
        }
    }

    pub fn rotated_obstacle(self, x: f32, z: f32, rotation: u8) -> Obstacle {
        let mut shape = self.obstacle(x, z);
        if rotation % 2 == 1 {
            std::mem::swap(&mut shape.half_x, &mut shape.half_z);
        }
        shape
    }

    pub fn build_samples(self, x: f32, z: f32) -> impl Iterator<Item = Point> {
        self.rotated_build_samples(x, z, 0)
    }

    /// Sample every occupied cell's corners and centre when checking level ground.
    pub fn rotated_build_samples(
        self,
        x: f32,
        z: f32,
        rotation: u8,
    ) -> impl Iterator<Item = Point> {
        let (width, depth) = self.rotated_grid_size(rotation);
        let shape = self.rotated_build_footprint(x, z, rotation);
        (0..=width * 2).flat_map(move |column| {
            (0..=depth * 2).map(move |row| {
                (
                    x - shape.half_x + column as f32 * BUILD_GRID_SIZE * 0.5,
                    z - shape.half_z + row as f32 * BUILD_GRID_SIZE * 0.5,
                )
            })
        })
    }

    pub fn blocks_construction_unit(self, x: f32, z: f32, point: Point) -> bool {
        self.rotated_blocks_construction_unit(x, z, 0, point)
    }

    /// Reserve the plot and ensure units will not be trapped inside model overhangs.
    pub fn rotated_blocks_construction_unit(
        self,
        x: f32,
        z: f32,
        rotation: u8,
        point: Point,
    ) -> bool {
        self.rotated_build_footprint(x, z, rotation)
            .contains(point, CLEARANCE)
            || self
                .rotated_obstacle(x, z, rotation)
                .contains(point, CLEARANCE)
    }
}

pub const CLEARANCE: f32 = 0.27;
pub const UNIT_SPACING: f32 = 0.55;
/// Room between formation slots allows units to settle without touching neighbours.
pub const FORMATION_SPACING: f32 = UNIT_SPACING * 2.0 + 0.1;
pub const ATTACK_RANGE: f32 = 0.8;
pub type Point = (f32, f32);

/// Parallel rows centered on the drag midpoint. Full rows span the drag;
/// a partial final row is centered with the same spacing as the full rows.
pub fn formation_positions(start: Point, end: Point, count: usize) -> Vec<Point> {
    if count == 0 {
        return Vec::new();
    }
    let length = distance(start, end);
    if !length.is_finite() || !start.0.is_finite() || !start.1.is_finite() {
        return Vec::new();
    }
    let along = if length > 0.0001 {
        ((end.0 - start.0) / length, (end.1 - start.1) / length)
    } else {
        (1.0, 0.0)
    };
    let columns = ((length / FORMATION_SPACING).floor() as usize)
        .saturating_add(1)
        .clamp(1, count);
    let rows = count.div_ceil(columns);
    let spacing = if columns > 1 {
        length / (columns - 1) as f32
    } else {
        0.0
    };
    let center = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
    (0..count)
        .map(|index| {
            let row = index / columns;
            let in_row = columns.min(count - row * columns);
            let x = (index % columns) as f32 * spacing - (in_row - 1) as f32 * spacing * 0.5;
            let z = (row as f32 - (rows - 1) as f32 * 0.5) * FORMATION_SPACING;
            (
                center.0 + along.0 * x - along.1 * z,
                center.1 + along.1 * x + along.0 * z,
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
pub struct AttackPosition {
    pub point: Point,
    pub side: usize,
    pub ring: usize,
}

/// Spaced positions on all four sides. Outer rings hold reinforcements when the
/// attacking perimeter is full; they never deal damage from outside melee range.
pub fn attack_positions(target: Obstacle) -> Vec<AttackPosition> {
    let mut positions = Vec::new();
    for ring in 0..4 {
        let extra = ring as f32 * 0.75;
        for side in 0..4 {
            let half = if side < 2 {
                target.half_z
            } else {
                target.half_x
            } + extra;
            let intervals = ((half * 2.0 / 0.7).floor() as usize).max(1);
            for index in 0..=intervals {
                let along = -half + half * 2.0 * index as f32 / intervals as f32;
                let point = match side {
                    0 => (target.x - target.half_x - 0.6 - extra, target.z + along),
                    1 => (target.x + target.half_x + 0.6 + extra, target.z + along),
                    2 => (target.x + along, target.z - target.half_z - 0.6 - extra),
                    _ => (target.x + along, target.z + target.half_z + 0.6 + extra),
                };
                positions.push(AttackPosition { point, side, ring });
            }
        }
    }
    positions
}

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub x: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
}
impl Obstacle {
    pub fn contains(self, p: Point, margin: f32) -> bool {
        (p.0 - self.x).abs() < self.half_x + margin && (p.1 - self.z).abs() < self.half_z + margin
    }
    pub fn overlaps(self, other: Self) -> bool {
        (self.x - other.x).abs() < self.half_x + other.half_x - 0.0001
            && (self.z - other.z).abs() < self.half_z + other.half_z - 0.0001
    }
    fn within(self, limit: f32) -> bool {
        self.x.is_finite()
            && self.z.is_finite()
            && self.x.abs() + self.half_x <= limit
            && self.z.abs() + self.half_z <= limit
    }
    pub fn in_map(self) -> bool {
        self.within(MAP_LIMIT)
    }
    pub fn in_build_area(self) -> bool {
        self.within(MAP_SIZE * 0.5)
    }
    pub fn closest(self, p: Point) -> Point {
        (
            p.0.clamp(self.x - self.half_x, self.x + self.half_x),
            p.1.clamp(self.z - self.half_z, self.z + self.half_z),
        )
    }
}
pub fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
pub fn free(p: Point, obstacles: &[Obstacle]) -> bool {
    free_with_clearance(p, obstacles, CLEARANCE)
}
fn free_with_clearance(p: Point, obstacles: &[Obstacle], clearance: f32) -> bool {
    p.0.is_finite()
        && p.1.is_finite()
        && p.0.abs() <= MAP_LIMIT
        && p.1.abs() <= MAP_LIMIT
        && !obstacles.iter().any(|b| b.contains(p, clearance))
}
// Slab intersection against an open rectangle. Touching its boundary is allowed.
pub fn segment_clear(a: Point, b: Point, obstacles: &[Obstacle]) -> bool {
    segment_clear_with_clearance(a, b, obstacles, CLEARANCE)
}
fn segment_clear_with_clearance(
    a: Point,
    b: Point,
    obstacles: &[Obstacle],
    clearance: f32,
) -> bool {
    !obstacles.iter().any(|o| {
        let mut lo = 0.0_f32;
        let mut hi = 1.0_f32;
        for (v, d, min, max) in [
            (
                a.0,
                b.0 - a.0,
                o.x - o.half_x - clearance,
                o.x + o.half_x + clearance,
            ),
            (
                a.1,
                b.1 - a.1,
                o.z - o.half_z - clearance,
                o.z + o.half_z + clearance,
            ),
        ] {
            if d.abs() < 1e-7 {
                if v <= min || v >= max {
                    return false;
                }
            } else {
                let t1 = (min - v) / d;
                let t2 = (max - v) / d;
                lo = lo.max(t1.min(t2));
                hi = hi.min(t1.max(t2));
            }
        }
        lo < hi && hi > 0.0 && lo < 1.0
    })
}
/// Visibility graph + Dijkstra gives the shortest polyline around axis-aligned
/// building footprints, with unit-radius clearance (no diagonal corner cutting).
pub fn shortest_path(start: Point, goal: Point, obstacles: &[Obstacle]) -> Option<Vec<Point>> {
    shortest_path_with_clearance(start, goal, obstacles, CLEARANCE)
}
pub fn shortest_path_with_clearance(
    start: Point,
    goal: Point,
    obstacles: &[Obstacle],
    clearance: f32,
) -> Option<Vec<Point>> {
    if !free_with_clearance(start, obstacles, clearance)
        || !free_with_clearance(goal, obstacles, clearance)
    {
        return None;
    }
    if segment_clear_with_clearance(start, goal, obstacles, clearance) {
        return Some(vec![goal]);
    }
    let mut nodes = vec![start, goal];
    for b in obstacles {
        for x in [
            b.x - b.half_x - clearance - 0.001,
            b.x + b.half_x + clearance + 0.001,
        ] {
            for z in [
                b.z - b.half_z - clearance - 0.001,
                b.z + b.half_z + clearance + 0.001,
            ] {
                if free_with_clearance((x, z), obstacles, clearance) {
                    nodes.push((x, z));
                }
            }
        }
    }
    visibility_path(&nodes, |a, b| {
        segment_clear_with_clearance(a, b, obstacles, clearance)
    })
}

/// Shared shortest-path search over visibility vertices. Start and goal are the
/// first two vertices; callers supply static or dynamic collision checks.
pub fn visibility_path(
    nodes: &[Point],
    clear: impl Fn(Point, Point) -> bool,
) -> Option<Vec<Point>> {
    let goal = *nodes.get(1)?;
    let n = nodes.len();
    let mut costs = vec![f32::INFINITY; n];
    let mut prev = vec![0; n];
    let mut visited = vec![false; n];
    costs[0] = 0.0;
    for _ in 0..n {
        let u = (0..n)
            .filter(|i| !visited[*i])
            .min_by(|a, b| costs[*a].total_cmp(&costs[*b]))?;
        if !costs[u].is_finite() {
            return None;
        }
        if u == 1 {
            let mut path = vec![goal];
            let mut i = 1;
            while prev[i] != 0 {
                i = prev[i];
                path.push(nodes[i]);
            }
            path.reverse();
            return Some(path);
        }
        visited[u] = true;
        for v in 1..n {
            let next = costs[u] + distance(nodes[u], nodes[v]);
            if !visited[v] && next < costs[v] && clear(nodes[u], nodes[v]) {
                costs[v] = next;
                prev[v] = u;
            }
        }
    }
    None
}

pub fn attack_path(start: Point, target: Obstacle, obstacles: &[Obstacle]) -> Option<Vec<Point>> {
    let mut candidates = Vec::new();
    for x in [
        target.x - target.half_x - 0.6,
        target.x + target.half_x + 0.6,
    ] {
        candidates.push((
            x,
            start
                .1
                .clamp(target.z - target.half_z, target.z + target.half_z),
        ));
        for z in [target.z - target.half_z, target.z + target.half_z] {
            candidates.push((x, z));
        }
    }
    for z in [
        target.z - target.half_z - 0.6,
        target.z + target.half_z + 0.6,
    ] {
        candidates.push((
            start
                .0
                .clamp(target.x - target.half_x, target.x + target.half_x),
            z,
        ));
        for x in [target.x - target.half_x, target.x + target.half_x] {
            candidates.push((x, z));
        }
    }
    candidates
        .into_iter()
        .filter_map(|goal| shortest_path(start, goal, obstacles))
        .min_by(|a, b| path_length(start, a).total_cmp(&path_length(start, b)))
}
pub fn path_length(start: Point, path: &[Point]) -> f32 {
    let mut p = start;
    path.iter()
        .map(|q| {
            let d = distance(p, *q);
            p = *q;
            d
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_plots_swap_dimensions_and_keep_all_edges_on_grid_lines() {
        for kind in BuildingKind::ALL {
            let (width, depth) = kind.grid_size();
            for rotation in 0..4 {
                let dimensions = kind.rotated_grid_size(rotation);
                assert_eq!(
                    dimensions,
                    if rotation % 2 == 0 {
                        (width, depth)
                    } else {
                        (depth, width)
                    }
                );
                let (x, z) = kind.rotated_snap_position(-13.7, 20.4, rotation);
                let plot = kind.rotated_build_footprint(x, z, rotation);
                for edge in [
                    x - plot.half_x,
                    x + plot.half_x,
                    z - plot.half_z,
                    z + plot.half_z,
                ] {
                    assert_eq!(edge % BUILD_GRID_SIZE, 0.0);
                }
                let normal = kind.obstacle(x, z);
                let turned = kind.rotated_obstacle(x, z, rotation);
                assert_eq!(
                    (turned.half_x, turned.half_z),
                    if rotation % 2 == 0 {
                        (normal.half_x, normal.half_z)
                    } else {
                        (normal.half_z, normal.half_x)
                    }
                );
                for (px, pz) in kind.rotated_build_samples(x, z, rotation) {
                    assert!(px >= x - plot.half_x && px <= x + plot.half_x);
                    assert!(pz >= z - plot.half_z && pz <= z + plot.half_z);
                }
            }
        }
        let kind = BuildingKind::Barracks;
        assert_eq!(kind.rotated_snap_position(100.3, 100.3, 0), (100.0, 101.0));
        assert_eq!(kind.rotated_snap_position(100.3, 100.3, 1), (101.0, 100.0));
        let turned = kind.rotated_build_footprint(101.0, 100.0, 1);
        assert!(!turned.overlaps(BuildingKind::TowerStairwell.build_footprint(108.0, 100.0)));
        assert!(kind
            .build_footprint(101.0, 100.0)
            .overlaps(BuildingKind::TowerStairwell.build_footprint(108.0, 100.0)));
    }

    #[test]
    fn adjacent_stair_towers_reserve_exactly_eight_by_four_metres() {
        let kind = BuildingKind::TowerStairwell;
        assert_eq!(kind.grid_size(), (2, 2));
        let first = kind.snap_position(100.8, 100.7);
        let second = kind.snap_position(104.3, 100.4);
        assert_eq!(first, (100.0, 100.0));
        assert_eq!(second, (104.0, 100.0));
        let a = kind.build_footprint(first.0, first.1);
        let b = kind.build_footprint(second.0, second.1);
        assert!(!a.overlaps(b));
        assert!(kind
            .obstacle(first.0, first.1)
            .overlaps(kind.obstacle(second.0, second.1)));
        assert_eq!((b.x + b.half_x) - (a.x - a.half_x), 8.0);
        assert_eq!(a.half_z * 2.0, 4.0);
        assert!(a.overlaps(kind.build_footprint(102.0, 100.0)));
        assert!(b.overlaps(kind.build_footprint(102.0, 100.0)));
    }

    #[test]
    fn all_building_plots_snap_their_edges_to_the_same_two_metre_grid() {
        for kind in BuildingKind::ALL {
            let (width, depth) = kind.grid_size();
            assert!(width > 0 && depth > 0);
            for requested in [(0.0, 0.0), (-3.8, -21.5), (7.3, 19.1), (105.9, -104.7)] {
                let (x, z) = kind.snap_position(requested.0, requested.1);
                assert_eq!(kind.snap_position(x, z), (x, z));
                let shape = kind.build_footprint(x, z);
                for edge in [
                    x - shape.half_x,
                    x + shape.half_x,
                    z - shape.half_z,
                    z + shape.half_z,
                ] {
                    assert_eq!(edge % BUILD_GRID_SIZE, 0.0, "{}", kind.name());
                }
                assert!((x - requested.0).abs() <= 1.0 && (z - requested.1).abs() <= 1.0);
                assert!(
                    !shape.overlaps(kind.build_footprint(x + width as f32 * BUILD_GRID_SIZE, z))
                );
                assert!(
                    !shape.overlaps(kind.build_footprint(x, z + depth as f32 * BUILD_GRID_SIZE))
                );
            }
        }
        let tower = BuildingKind::TowerStairwell;
        assert!(tower.build_footprint(398.0, 398.0).in_build_area());
        assert!(!tower.build_footprint(400.0, 398.0).in_build_area());
        let invalid = tower.snap_position(f32::NAN, 0.0);
        assert!(!tower.build_footprint(invalid.0, invalid.1).in_build_area());
        let invalid = tower.snap_position(f32::INFINITY, 0.0);
        assert!(!tower.build_footprint(invalid.0, invalid.1).in_build_area());
    }

    #[test]
    fn catalog_has_valid_geometry_and_attack_slots_for_every_building() {
        assert_eq!(
            BuildingKind::ALL
                .iter()
                .filter(|kind| kind.category() == "fortifications")
                .count(),
            25
        );
        for (index, kind) in BuildingKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), index);
            assert_eq!(BuildingKind::from_name(kind.name()), Some(kind));
            let shape = kind.obstacle(100.0, 100.0);
            assert!(shape.in_map(), "{}", kind.name());
            assert!(shape.half_x > 0.0 && shape.half_z > 0.0);
            for slot in attack_positions(shape)
                .into_iter()
                .filter(|slot| slot.ring == 0)
            {
                assert!(slot.point.0.is_finite() && slot.point.1.is_finite());
                assert!(
                    (distance(slot.point, shape.closest(slot.point)) - 0.6).abs() < 0.0001,
                    "{}",
                    kind.name()
                );
            }
        }
        assert_eq!(BuildingKind::from_name("unknown"), None);
        assert!(free((390.0, 390.0), &[]));
        assert!(!free((400.0, 0.0), &[]));
    }

    #[test]
    fn different_buildings_use_their_own_bounds_for_overlap_paths_and_attacks() {
        let granary = BuildingKind::Granary.obstacle(0.0, 0.0);
        let barracks = BuildingKind::Barracks.obstacle(8.0, 0.0);
        assert!(granary.overlaps(barracks));
        assert!(barracks.overlaps(granary));
        assert!(!granary.overlaps(BuildingKind::Granary.obstacle(8.0, 0.0)));
        assert!(!BuildingKind::Barracks
            .obstacle(MAP_LIMIT - 5.0, 0.0)
            .in_map());
        let barracks = BuildingKind::Barracks.obstacle(0.0, 0.0);
        assert!(barracks.contains((5.0, 0.0), CLEARANCE));
        assert!(!granary.contains((5.0, 0.0), CLEARANCE));
        let mut start = (-10.0, 0.0);
        let path = shortest_path(start, (10.0, 0.0), &[barracks]).unwrap();
        assert_eq!(path.len(), 3);
        for point in path {
            assert!(segment_clear(start, point, &[barracks]));
            start = point;
        }
        for slot in attack_positions(barracks)
            .into_iter()
            .filter(|s| s.ring == 0)
        {
            assert!((distance(slot.point, barracks.closest(slot.point)) - 0.6).abs() < 0.0001);
            assert!(free(slot.point, &[barracks]));
        }
    }

    #[test]
    fn formation_rows_span_drag_and_center_partial_row() {
        let positions = formation_positions((2.0, 3.0), (6.0, 3.0), 10);
        assert_eq!(positions.len(), 10);
        // Four columns, three rows; full rows span the exact drag width.
        assert_eq!(positions[0], (2.0, 3.0 - FORMATION_SPACING));
        assert_eq!(positions[3], (6.0, 3.0 - FORMATION_SPACING));
        assert_eq!(positions[4], (2.0, 3.0));
        assert_eq!(positions[7], (6.0, 3.0));
        assert!(((positions[8].0 + positions[9].0) * 0.5 - 4.0).abs() < 0.0001);
        assert_eq!(positions[9].1, 3.0 + FORMATION_SPACING);
        for (i, a) in positions.iter().enumerate() {
            for b in &positions[i + 1..] {
                assert!(distance(*a, *b) >= UNIT_SPACING - 0.0001);
            }
        }
    }

    #[test]
    fn formation_rotates_with_drag_and_handles_short_drags() {
        let horizontal = formation_positions((-1.0, 0.0), (1.0, 0.0), 10);
        let vertical = formation_positions((0.0, -1.0), (0.0, 1.0), 10);
        for (a, b) in horizontal.iter().zip(vertical) {
            assert!((a.0 - b.1).abs() < 0.0001);
            assert!((a.1 + b.0).abs() < 0.0001);
        }
        assert_eq!(
            formation_positions((0.0, 0.0), (2.0, 2.0), 1),
            vec![(1.0, 1.0)]
        );
        assert!(formation_positions((0.0, 0.0), (1.0, 0.0), 0).is_empty());
        let column = formation_positions((2.0, 3.0), (2.0, 3.0), 3);
        assert_eq!(
            column,
            vec![
                (2.0, 3.0 - FORMATION_SPACING),
                (2.0, 3.0),
                (2.0, 3.0 + FORMATION_SPACING)
            ]
        );
        assert!(formation_positions((f32::NAN, 0.0), (0.0, 0.0), 3).is_empty());
    }

    #[test]
    fn routes_around_building_without_cutting_corners() {
        let obstacles = [BuildingKind::Granary.obstacle(0.0, 0.0)];
        let start = (-8.0, 0.0);
        let goal = (8.0, 0.0);
        let path = shortest_path(start, goal, &obstacles).unwrap();
        assert_eq!(path.len(), 3);
        let mut p = start;
        for q in &path {
            assert!(segment_clear(p, *q, &obstacles));
            p = *q;
        }
        let expected = 2.0 * distance(start, (-HALF_X - CLEARANCE, -HALF_Z - CLEARANCE))
            + 2.0 * (HALF_X + CLEARANCE);
        assert!((path_length(start, &path) - expected).abs() < 0.01);
        assert!(shortest_path(start, (0.0, 0.0), &obstacles).is_none());
    }
    #[test]
    fn wall_blocks_routes_and_attack_stops_outside_collision() {
        let wall = [Obstacle {
            half_z: MAP_LIMIT + 1.0,
            ..BuildingKind::Granary.obstacle(0.0, 0.0)
        }];
        assert!(shortest_path((-8.0, 0.0), (8.0, 0.0), &wall).is_none());
        let target = BuildingKind::Granary.obstacle(0.0, 0.0);
        let path = attack_path((-8.0, 0.0), target, &[target]).unwrap();
        let end = *path.last().unwrap();
        assert!((distance(end, target.closest(end)) - 0.6).abs() < 0.001);
    }
    #[test]
    fn collision_footprint_overlap_and_map_bounds() {
        let a = BuildingKind::Granary.obstacle(0.0, 0.0);
        assert!(a.overlaps(BuildingKind::Granary.obstacle(5.0, 0.0)));
        assert!(!a.overlaps(BuildingKind::Granary.obstacle(HALF_X * 2.0, 0.0)));
        assert!(!BuildingKind::Granary
            .obstacle(MAP_LIMIT - 1.0, 0.0)
            .in_map());
    }
}

pub mod units;

pub mod combat;
pub mod ranged;

impl BuildingKind {
    pub fn is_tower(self) -> bool {
        matches!(
            self,
            Self::TowerWooden
                | Self::TowerLookout
                | Self::TowerStairwell
                | Self::TowerSquare
                | Self::TowerRoundShort
                | Self::TowerRoundTall
                | Self::TowerGreat
                | Self::Watchpost
                | Self::FlagTower
                | Self::TowerWalk
        )
    }
    pub fn is_anchor(self) -> bool {
        self.is_tower() || self.is_gate()
    }
}
