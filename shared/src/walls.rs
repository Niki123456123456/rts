//! The same raster line and side-connected wall graph are used by client and server.
use crate::{building_roof::BuildingRoof, BuildingKind, Obstacle, Point};
use std::collections::{HashMap, VecDeque};
pub const WALL_HEIGHT: f32 = 6.663;
pub const MAX_WALL_SEGMENTS: usize = 4096;

/// Inclusive, four-connected raster line. Diagonals become gap-free grid steps.
pub fn wall_line(start: Point, end: Point) -> Vec<Point> {
    if ![start.0, start.1, end.0, end.1]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= crate::MAP_LIMIT + 2.)
    {
        return Vec::new();
    }
    let snap = |p: Point| BuildingKind::StoneWall.snap_position(p.0, p.1);
    let (start, end) = (snap(start), snap(end));
    let (mut x, mut z) = (start.0 as i32, start.1 as i32);
    let (dx, dz) = ((end.0 - start.0) as i32 / 2, (end.1 - start.1) as i32 / 2);
    let (nx, nz) = (dx.abs(), dz.abs());
    let (mut ix, mut iz) = (0, 0);
    let mut points = vec![(x as f32, z as f32)];
    while ix < nx || iz < nz {
        if ix < nx && (iz == nz || (2 * ix + 1) * nz <= (2 * iz + 1) * nx) {
            x += dx.signum() * 2;
            ix += 1;
        } else {
            z += dz.signum() * 2;
            iz += 1;
        }
        points.push((x as f32, z as f32));
    }
    points
}

#[derive(Clone, Copy, Debug)]
pub struct RoofPiece {
    pub id: u64,
    pub kind: BuildingKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: u8,
}
impl RoofPiece {
    pub fn roof(self) -> BuildingRoof {
        BuildingRoof::new(self.kind, self.x, self.y, self.z, self.rotation).unwrap()
    }
    fn plot(self) -> Obstacle {
        self.kind
            .rotated_build_footprint(self.x, self.z, self.rotation)
    }
}
/// Only full side contact connects walls; diagonal corner contact never does.
pub fn portal(a: RoofPiece, b: RoofPiece) -> Option<Point> {
    if (a.y - b.y).abs() > 0.05 {
        return None;
    }
    let (a, b) = (a.plot(), b.plot());
    let overlap_x = (a.x + a.half_x).min(b.x + b.half_x) - (a.x - a.half_x).max(b.x - b.half_x);
    let overlap_z = (a.z + a.half_z).min(b.z + b.half_z) - (a.z - a.half_z).max(b.z - b.half_z);
    if ((a.x - b.x).abs() - a.half_x - b.half_x).abs() < 0.01 && overlap_z >= 1.4 {
        Some((
            (a.x + a.half_x).min(b.x + b.half_x),
            ((a.z - a.half_z).max(b.z - b.half_z) + (a.z + a.half_z).min(b.z + b.half_z)) * 0.5,
        ))
    } else if ((a.z - b.z).abs() - a.half_z - b.half_z).abs() < 0.01 && overlap_x >= 1.4 {
        Some((
            ((a.x - a.half_x).max(b.x - b.half_x) + (a.x + a.half_x).min(b.x + b.half_x)) * 0.5,
            (a.z + a.half_z).min(b.z + b.half_z),
        ))
    } else {
        None
    }
}
fn wall_tree(target: u64, pieces: &[RoofPiece]) -> HashMap<u64, Option<u64>> {
    let walls: HashMap<_, _> = pieces
        .iter()
        .filter(|p| p.kind == BuildingKind::StoneWall)
        .map(|p| ((p.x.round() as i32, p.z.round() as i32), p))
        .collect();
    let by_id: HashMap<_, _> = pieces.iter().map(|p| (p.id, p)).collect();
    let mut parents = HashMap::new();
    let mut queue = VecDeque::from([target]);
    parents.insert(target, None);
    while let Some(id) = queue.pop_front() {
        let Some(a) = by_id.get(&id) else {
            continue;
        };
        for (dx, dz) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
            let Some(b) = walls.get(&(a.x.round() as i32 + dx, a.z.round() as i32 + dz)) else {
                continue;
            };
            if !parents.contains_key(&b.id) && portal(**a, **b).is_some() {
                parents.insert(b.id, Some(id));
                queue.push_back(b.id);
            }
        }
    }
    parents
}

fn chain(
    mut id: u64,
    tree: &HashMap<u64, Option<u64>>,
    pieces: &[RoofPiece],
) -> Option<Vec<Point>> {
    let mut points = Vec::new();
    loop {
        let p = pieces.iter().find(|p| p.id == id)?;
        points.push((p.x, p.z));
        match tree.get(&id)? {
            Some(next) => id = *next,
            None => return Some(points),
        }
    }
}
pub fn wall_component(target: u64, pieces: &[RoofPiece]) -> Vec<RoofPiece> {
    let tree = wall_tree(target, pieces);
    let mut result = pieces
        .iter()
        .filter(|p| p.kind == BuildingKind::StoneWall && tree.contains_key(&p.id))
        .copied()
        .collect::<Vec<_>>();
    result.sort_by_key(|p| p.id);
    result
}

/// Centres along a wall-only path, including both endpoints.
pub fn wall_path(start: u64, target: u64, pieces: &[RoofPiece]) -> Option<Vec<Point>> {
    chain(start, &wall_tree(target, pieces), pieces)
}
/// Every reachable tower and its route from the hatch to the target wall centre.
pub fn tower_routes(target: u64, pieces: &[RoofPiece]) -> Vec<(u64, Vec<Point>)> {
    let tree = wall_tree(target, pieces);
    let mut routes = Vec::new();
    for tower in pieces
        .iter()
        .filter(|p| p.kind != BuildingKind::StoneWall && p.kind.roof_profile().is_some())
    {
        for wall in pieces
            .iter()
            .filter(|p| p.kind == BuildingKind::StoneWall && tree.contains_key(&p.id))
        {
            if let Some(portal) = portal(*tower, *wall) {
                let mut points = vec![tower.roof().hatch(), portal];
                points.extend(chain(wall.id, &tree, pieces).unwrap());
                routes.push((tower.id, points));
            }
        }
    }
    routes.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then(a.0.cmp(&b.0)));
    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn piece(id: u64, kind: BuildingKind, x: f32, z: f32) -> RoofPiece {
        RoofPiece {
            id,
            kind,
            x,
            y: 0.,
            z,
            rotation: 0,
        }
    }
    #[test]
    fn raster_lines_are_inclusive_connected_and_bounded() {
        for (a, b) in [
            ((1., 1.), (9., 7.)),
            ((9., 7.), (1., 1.)),
            ((1., 1.), (1., 1.)),
            ((-7., 5.), (9., -9.)),
        ] {
            let line = wall_line(a, b);
            assert_eq!(line.first(), Some(&a));
            assert_eq!(line.last(), Some(&b));
            for p in line.windows(2) {
                assert_eq!((p[0].0 - p[1].0).abs() + (p[0].1 - p[1].1).abs(), 2.);
            }
        }
        assert!(wall_line((f32::NAN, 0.), (0., 0.)).is_empty());
        assert!(wall_line((0., 0.), (1e30, 0.)).is_empty());
    }
    #[test]
    fn walls_require_side_connected_tower_and_disconnection_breaks_routes() {
        let mut pieces = vec![
            piece(1, BuildingKind::TowerStairwell, 0., 0.),
            piece(2, BuildingKind::StoneWall, 3., 1.),
            piece(3, BuildingKind::StoneWall, 5., 1.),
            piece(4, BuildingKind::StoneWall, 5., 3.),
        ];
        assert_eq!(tower_routes(4, &pieces)[0].0, 1);
        assert_eq!(
            wall_path(2, 4, &pieces).unwrap(),
            vec![(3., 1.), (5., 1.), (5., 3.)]
        );
        pieces.remove(2);
        assert!(tower_routes(4, &pieces).is_empty());
        assert!(wall_path(2, 4, &pieces).is_none());
        assert_eq!(
            BuildingKind::StoneWall.roof_profile().unwrap().capacity(),
            9
        );
        assert_eq!(
            BuildingKind::StoneWall.roof_profile().unwrap().height,
            BuildingKind::TowerStairwell.roof_profile().unwrap().height
        );
    }
}

/// Physical roof network; combat routes can cross ownership boundaries.
pub fn surface_component(start: u64, pieces: &[RoofPiece]) -> Vec<RoofPiece> {
    let mut ids = std::collections::HashSet::from([start]);
    let mut queue = vec![start];
    let mut index = 0;
    while index < queue.len() {
        if let Some(a) = pieces.iter().find(|p| p.id == queue[index]) {
            for b in pieces {
                if !ids.contains(&b.id)
                    && (a.kind == BuildingKind::StoneWall || b.kind == BuildingKind::StoneWall)
                    && portal(*a, *b).is_some()
                {
                    ids.insert(b.id);
                    queue.push(b.id);
                }
            }
        }
        index += 1;
    }
    pieces
        .iter()
        .filter(|p| ids.contains(&p.id))
        .copied()
        .collect()
}
/// A short melee segment must have floor beneath it throughout, never cross a gap.
pub fn surface_supports(p: (f32, f32, f32), pieces: &[RoofPiece]) -> bool {
    pieces.iter().any(|piece| {
        let roof = piece.roof();
        let (x, z) = roof.local((p.0, p.2));
        (p.1 - roof.roof_y() - crate::building_roof::UNIT_CENTER_HEIGHT).abs() < 0.1
            && x.abs() <= roof.profile.half_x + crate::CLEARANCE
            && z.abs() <= roof.profile.half_z + crate::CLEARANCE
    })
}
pub fn surface_contact(a: (f32, f32, f32), b: (f32, f32, f32), pieces: &[RoofPiece]) -> bool {
    (0..=12).all(|i| {
        let f = i as f32 / 12.;
        surface_supports(
            (
                a.0 + (b.0 - a.0) * f,
                a.1 + (b.1 - a.1) * f,
                a.2 + (b.2 - a.2) * f,
            ),
            pieces,
        )
    })
}

#[cfg(test)]
mod combat_surface_tests {
    use super::*;
    #[test]
    fn melee_crosses_joined_wall_pieces_but_never_a_gap_or_another_floor() {
        let pieces = [
            RoofPiece {
                id: 1,
                kind: BuildingKind::TowerStairwell,
                x: 0.,
                y: 0.,
                z: 0.,
                rotation: 0,
            },
            RoofPiece {
                id: 2,
                kind: BuildingKind::StoneWall,
                x: 3.,
                y: 0.,
                z: 1.,
                rotation: 0,
            },
            RoofPiece {
                id: 3,
                kind: BuildingKind::StoneWall,
                x: 5.,
                y: 0.,
                z: 1.,
                rotation: 0,
            },
        ];
        assert_eq!(surface_component(3, &pieces).len(), 3);
        let y = WALL_HEIGHT + crate::building_roof::UNIT_CENTER_HEIGHT;
        assert!(surface_contact((3.7, y, 1.), (4.3, y, 1.), &pieces));
        assert!(!surface_contact((3.7, 0.75, 1.), (4.3, y, 1.), &pieces));
        assert!(!surface_contact(
            (1.7, y, 1.),
            (4.3, y, 1.),
            &pieces[..1]
                .iter()
                .chain(pieces[2..].iter())
                .copied()
                .collect::<Vec<_>>()
        ));
    }
}
