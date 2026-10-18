//! Shared building catalog, collision footprints and continuous shortest-path navigation.
//! Bounds come from POSITION accessor 0 of each collision GLB, scaled 1:100.
pub const MODEL_SCALE: f32 = 0.01;
pub const MODEL_CENTER: (f32, f32) = (308.07544, -313.6961);
pub const HALF_X: f32 = (581.97156 - 34.17932) * MODEL_SCALE / 2.0;
pub const HALF_Z: f32 = (567.28033 - 60.11186) * MODEL_SCALE / 2.0;
/// Buildable models and their collision bounds (source units, before 1:100 scale).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildingKind {
    #[default]
    Granary,
    Barracks,
    Stockpile,
}
impl BuildingKind {
    pub const ALL: [Self; 3] = [Self::Granary, Self::Barracks, Self::Stockpile];
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "granary" => Some(Self::Granary),
            "barracks" => Some(Self::Barracks),
            "stockpile" => Some(Self::Stockpile),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Granary => "granary",
            Self::Barracks => "barracks",
            Self::Stockpile => "stockpile",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Granary => "Kornspeicher",
            Self::Barracks => "Steinkaserne",
            Self::Stockpile => "Vorratslager",
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Granary => 0,
            Self::Barracks => 1,
            Self::Stockpile => 2,
        }
    }
    pub fn scene(self) -> &'static str {
        match self {
            Self::Granary => "models/buildings/supply/granary/mesh.glb#Scene0",
            Self::Barracks => "models/buildings/military/barrack_stone/mesh.glb#Scene0",
            Self::Stockpile => "models/buildings/supply/stockpile/mesh.glb#Scene0",
        }
    }
    pub fn collision(self) -> &'static str {
        match self {
            Self::Granary => "models/buildings/supply/granary/collision.glb#Mesh0/Primitive0",
            Self::Stockpile => "models/buildings/supply/stockpile/collision.glb#Mesh0/Primitive0",
            Self::Barracks => {
                "models/buildings/military/barrack_stone/collision.glb#Mesh0/Primitive0"
            }
        }
    }
    pub fn center(self) -> Point {
        match self {
            Self::Granary => MODEL_CENTER,
            Self::Stockpile => (
                (-5.0412598 + 1438.4318) / 2.0,
                (-1043.6819 + 27.415283) / 2.0,
            ),
            Self::Barracks => (1231.5162 / 2.0, (-1025.0 + 2.5813599) / 2.0),
        }
    }
    pub fn obstacle(self, x: f32, z: f32) -> Obstacle {
        let (half_x, half_z) = match self {
            Self::Granary => (HALF_X, HALF_Z),
            Self::Stockpile => (
                (1438.4318 + 5.0412598) * MODEL_SCALE / 2.0,
                (1043.6819 + 27.415283) * MODEL_SCALE / 2.0,
            ),
            Self::Barracks => (
                1231.5162 * MODEL_SCALE / 2.0,
                (1025.0 + 2.5813599) * MODEL_SCALE / 2.0,
            ),
        };
        Obstacle {
            x,
            z,
            half_x,
            half_z,
        }
    }
}
pub const MAP_LIMIT: f32 = 39.0;
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
            let intervals = (half * 2.0 / 0.7).floor() as usize;
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
        (self.x - other.x).abs() < self.half_x + other.half_x
            && (self.z - other.z).abs() < self.half_z + other.half_z
    }
    pub fn in_map(self) -> bool {
        self.x.is_finite()
            && self.z.is_finite()
            && self.x.abs() + self.half_x <= MAP_LIMIT
            && self.z.abs() + self.half_z <= MAP_LIMIT
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
    fn different_buildings_use_their_own_bounds_for_overlap_paths_and_attacks() {
        let granary = BuildingKind::Granary.obstacle(0.0, 0.0);
        let barracks = BuildingKind::Barracks.obstacle(8.0, 0.0);
        assert!(granary.overlaps(barracks));
        assert!(barracks.overlaps(granary));
        assert!(!granary.overlaps(BuildingKind::Granary.obstacle(8.0, 0.0)));
        assert!(!BuildingKind::Barracks.obstacle(34.0, 0.0).in_map());
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
        let wall: Vec<_> = (-8..=8)
            .map(|i| BuildingKind::Granary.obstacle(0.0, i as f32 * 5.1))
            .collect();
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
        assert!(!BuildingKind::Granary.obstacle(38.0, 0.0).in_map());
    }
}
