//! Shared granary collision footprint and continuous shortest-path navigation.
//! Bounds come from POSITION accessor 0 of granary_collision.glb, scaled 1:100.
pub const MODEL_SCALE: f32 = 0.01;
pub const MODEL_CENTER: (f32, f32) = (308.07544, -313.6961);
pub const HALF_X: f32 = (581.97156 - 34.17932) * MODEL_SCALE / 2.0;
pub const HALF_Z: f32 = (567.28033 - 60.11186) * MODEL_SCALE / 2.0;
pub const MAP_LIMIT: f32 = 39.0;
pub const CLEARANCE: f32 = 0.27;
pub const UNIT_SPACING: f32 = 0.55;
pub const ATTACK_RANGE: f32 = 0.8;
pub type Point = (f32, f32);

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
            let half = if side < 2 { HALF_Z } else { HALF_X } + extra;
            let intervals = (half * 2.0 / 0.7).floor() as usize;
            for index in 0..=intervals {
                let along = -half + half * 2.0 * index as f32 / intervals as f32;
                let point = match side {
                    0 => (target.x - HALF_X - 0.6 - extra, target.z + along),
                    1 => (target.x + HALF_X + 0.6 + extra, target.z + along),
                    2 => (target.x + along, target.z - HALF_Z - 0.6 - extra),
                    _ => (target.x + along, target.z + HALF_Z + 0.6 + extra),
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
}
impl Obstacle {
    pub fn contains(self, p: Point, margin: f32) -> bool {
        (p.0 - self.x).abs() < HALF_X + margin && (p.1 - self.z).abs() < HALF_Z + margin
    }
    pub fn overlaps(self, other: Self) -> bool {
        (self.x - other.x).abs() < HALF_X * 2.0 && (self.z - other.z).abs() < HALF_Z * 2.0
    }
    pub fn in_map(self) -> bool {
        self.x.is_finite()
            && self.z.is_finite()
            && self.x.abs() + HALF_X <= MAP_LIMIT
            && self.z.abs() + HALF_Z <= MAP_LIMIT
    }
    pub fn closest(self, p: Point) -> Point {
        (
            p.0.clamp(self.x - HALF_X, self.x + HALF_X),
            p.1.clamp(self.z - HALF_Z, self.z + HALF_Z),
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
                o.x - HALF_X - clearance,
                o.x + HALF_X + clearance,
            ),
            (
                a.1,
                b.1 - a.1,
                o.z - HALF_Z - clearance,
                o.z + HALF_Z + clearance,
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
            b.x - HALF_X - clearance - 0.001,
            b.x + HALF_X + clearance + 0.001,
        ] {
            for z in [
                b.z - HALF_Z - clearance - 0.001,
                b.z + HALF_Z + clearance + 0.001,
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
    for x in [target.x - HALF_X - 0.6, target.x + HALF_X + 0.6] {
        candidates.push((x, start.1.clamp(target.z - HALF_Z, target.z + HALF_Z)));
        for z in [target.z - HALF_Z, target.z + HALF_Z] {
            candidates.push((x, z));
        }
    }
    for z in [target.z - HALF_Z - 0.6, target.z + HALF_Z + 0.6] {
        candidates.push((start.0.clamp(target.x - HALF_X, target.x + HALF_X), z));
        for x in [target.x - HALF_X, target.x + HALF_X] {
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
    fn routes_around_building_without_cutting_corners() {
        let obstacles = [Obstacle { x: 0.0, z: 0.0 }];
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
            .map(|i| Obstacle {
                x: 0.0,
                z: i as f32 * 5.1,
            })
            .collect();
        assert!(shortest_path((-8.0, 0.0), (8.0, 0.0), &wall).is_none());
        let target = Obstacle { x: 0.0, z: 0.0 };
        let path = attack_path((-8.0, 0.0), target, &[target]).unwrap();
        let end = *path.last().unwrap();
        assert!((distance(end, target.closest(end)) - 0.6).abs() < 0.001);
    }
    #[test]
    fn collision_footprint_overlap_and_map_bounds() {
        let a = Obstacle { x: 0.0, z: 0.0 };
        assert!(a.overlaps(Obstacle { x: 5.0, z: 0.0 }));
        assert!(!a.overlaps(Obstacle {
            x: HALF_X * 2.0,
            z: 0.0
        }));
        assert!(!Obstacle { x: 38.0, z: 0.0 }.in_map());
    }
}
