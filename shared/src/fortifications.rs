//! Physical gate passages, bridge spans and excavation footprints, shared by both clients.
use crate::{BuildingKind, Obstacle, Point};
pub fn world(origin: Point, rotation: u8, p: Point) -> Point {
    let p = match rotation % 4 {
        0 => p,
        1 => (p.1, -p.0),
        2 => (-p.0, -p.1),
        _ => (-p.1, p.0),
    };
    (origin.0 + p.0, origin.1 + p.1)
}
pub fn rect(origin: Point, rotation: u8, p: Point, hx: f32, hz: f32) -> Obstacle {
    let p = world(origin, rotation, p);
    let (hx, hz) = if rotation % 2 == 0 {
        (hx, hz)
    } else {
        (hz, hx)
    };
    Obstacle {
        x: p.0,
        z: p.1,
        half_x: hx,
        half_z: hz,
    }
}
// River navigation uses conservative bank rectangles. Extend the cut over dry bank approaches,
// while the physical deck remains exactly the rendered 36-metre span.
pub fn bridge_span(origin: Point, rotation: u8) -> Obstacle {
    rect(origin, rotation, (0., 17.5), 1.9, 28.5)
}
pub fn bridge_deck(origin: Point, rotation: u8) -> Obstacle {
    rect(origin, rotation, (0., 17.5), 1.9, 17.5)
}
pub fn gate_solids(kind: BuildingKind, origin: Point, rotation: u8, open: bool) -> Vec<Obstacle> {
    let shape = kind.rotated_obstacle(origin.0, origin.1, rotation);
    if !kind.is_gate() {
        return vec![shape];
    }
    let local = kind.obstacle(0., 0.);
    let hx = if kind == BuildingKind::DrawbridgeGate {
        4.9
    } else {
        local.half_x
    };
    let hz = if kind == BuildingKind::DrawbridgeGate {
        3.75
    } else {
        local.half_z
    };
    let passage = 1.8_f32.min(hx * 0.48);
    let mut result = vec![
        rect(
            origin,
            rotation,
            (-(hx + passage) * 0.5, 0.),
            (hx - passage) * 0.5,
            hz,
        ),
        rect(
            origin,
            rotation,
            ((hx + passage) * 0.5, 0.),
            (hx - passage) * 0.5,
            hz,
        ),
    ];
    if !open {
        result.push(rect(origin, rotation, (0., 0.), passage, 0.55));
    }
    if kind == BuildingKind::DrawbridgeGate {
        result.push(rect(origin, rotation, (6.4, -3.5), 1.3, 6.0));
    }
    result
}
/// Split a water rectangle around a traversable deck; never remove unrelated riverbanks.
pub fn subtract(a: Obstacle, cut: Obstacle) -> Vec<Obstacle> {
    let (l, r, b, t) = (
        a.x - a.half_x,
        a.x + a.half_x,
        a.z - a.half_z,
        a.z + a.half_z,
    );
    let (cl, cr, cb, ct) = (
        (cut.x - cut.half_x).max(l),
        (cut.x + cut.half_x).min(r),
        (cut.z - cut.half_z).max(b),
        (cut.z + cut.half_z).min(t),
    );
    if cl >= cr || cb >= ct {
        return vec![a];
    }
    [
        (l, cl, b, t),
        (cr, r, b, t),
        (cl, cr, b, cb),
        (cl, cr, ct, t),
    ]
    .into_iter()
    .filter(|(l, r, b, t)| r - l > 0.001 && t - b > 0.001)
    .map(|(l, r, b, t)| Obstacle {
        x: (l + r) * 0.5,
        z: (b + t) * 0.5,
        half_x: (r - l) * 0.5,
        half_z: (t - b) * 0.5,
    })
    .collect()
}
pub fn moat_line(a: Point, b: Point) -> Vec<Point> {
    crate::walls::wall_line((a.0 + 1., a.1 + 1.), (b.0 + 1., b.1 + 1.))
        .into_iter()
        .map(|p| (p.0 - 1., p.1 - 1.))
        .collect()
}
pub fn stair_bounds(base: Point, top: Point) -> Obstacle {
    Obstacle {
        x: (base.0 + top.0) * 0.5,
        z: (base.1 + top.1) * 0.5,
        half_x: (base.0 - top.0).abs() * 0.5 + 0.85,
        half_z: (base.1 - top.1).abs() * 0.5 + 0.85,
    }
}
pub fn merlons(origin: Point, rotation: u8) -> Vec<Obstacle> {
    let mut result = Vec::new();
    for x in [-0.86, 0.86] {
        for z in [-0.7, 0.7] {
            result.push(rect(origin, rotation, (x, z), 0.17, 0.275));
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opened_gates_leave_a_real_passage() {
        for kind in [
            BuildingKind::GatehouseSmall,
            BuildingKind::GatehouseLarge,
            BuildingKind::DrawbridgeGate,
        ] {
            for r in 0..4 {
                let a = world((0., 0.), r, (0., -9.));
                let b = world((0., 0.), r, (0., 9.));
                assert!(crate::segment_clear(
                    a,
                    b,
                    &gate_solids(kind, (0., 0.), r, true)
                ));
                assert!(!crate::segment_clear(
                    a,
                    b,
                    &gate_solids(kind, (0., 0.), r, false)
                ));
            }
        }
    }
    #[test]
    fn deck_only_cuts_its_own_corridor() {
        let water = Obstacle {
            x: 0.,
            z: 0.,
            half_x: 20.,
            half_z: 10.,
        };
        let cut = Obstacle {
            x: 0.,
            z: 0.,
            half_x: 2.,
            half_z: 12.,
        };
        let parts = subtract(water, cut);
        assert!(crate::segment_clear((0., -12.), (0., 12.), &parts));
        assert!(!crate::segment_clear((5., -12.), (5., 12.), &parts));
    }
}

/// Parallel stairs run alongside the wall, with a short horizontal upper landing.
pub fn stair_top(base: Point, wall: Point, height: f32) -> Point {
    let dx = base.0 - wall.0;
    let dz = base.1 - wall.1;
    if dx.abs() >= 2.4 && dx.abs() <= 5.0 && dz.abs() >= height * 1.2 {
        return (base.0, wall.1);
    }
    if dz.abs() >= 2.4 && dz.abs() <= 5.0 && dx.abs() >= height * 1.2 {
        return (wall.0, base.1);
    }
    let d = dx.hypot(dz).max(0.01);
    (wall.0 + dx / d * 2.2, wall.1 + dz / d * 2.2)
}
#[derive(Clone, Copy, Debug)]
pub struct Structure {
    pub id: u64,
    pub kind: BuildingKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: u8,
    pub owner: u32,
}
impl Structure {
    pub fn plot(self) -> Obstacle {
        self.kind
            .rotated_build_footprint(self.x, self.z, self.rotation)
    }
    pub fn body(self) -> Obstacle {
        self.kind.rotated_obstacle(self.x, self.z, self.rotation)
    }
    pub fn level(self) -> f32 {
        self.y
            + self
                .kind
                .roof_profile()
                .map_or_else(|| self.kind.height(), |p| p.height)
    }
}
pub fn covered_wall(point: Point, b: Structure) -> bool {
    (b.kind.is_anchor() && b.plot().contains(point, 0.))
        || (b.kind.is_wall() && crate::distance(point, (b.x, b.z)) < 0.1)
}
pub fn replaced_wall(new: Structure, old: Structure) -> bool {
    new.owner == old.owner
        && new.kind.is_anchor()
        && old.kind.is_wall()
        && new.plot().contains((old.x, old.z), 0.)
}
fn side_contact(a: Obstacle, b: Obstacle) -> bool {
    let ox = (a.x + a.half_x).min(b.x + b.half_x) - (a.x - a.half_x).max(b.x - b.half_x);
    let oz = (a.z + a.half_z).min(b.z + b.half_z) - (a.z - a.half_z).max(b.z - b.half_z);
    (ox.abs() < 0.02 && oz >= 0.6) || (oz.abs() < 0.02 && ox >= 0.6)
}
pub fn may_join(a: Structure, b: Structure) -> bool {
    a.owner == b.owner
        && (a.kind.is_anchor() || b.kind.is_anchor())
        && (a.kind.is_anchor() || a.kind.is_wall())
        && (b.kind.is_anchor() || b.kind.is_wall())
        && side_contact(a.plot(), b.plot())
}
#[derive(Clone, Copy, Debug)]
pub struct FortJoin {
    pub a: u64,
    pub b: u64,
    pub shape: Obstacle,
    pub base: f32,
    pub top: f32,
    pub wood: bool,
}
fn connection_axis(a: Structure, b: Structure) -> Option<bool> {
    if a.owner != b.owner {
        return None;
    }
    let eligible = (a.kind.is_anchor() && b.kind.is_anchor())
        || (a.kind.is_wall() && !b.kind.is_wall())
        || (b.kind.is_wall() && !a.kind.is_wall());
    if !eligible {
        return None;
    }
    let (p, q) = (a.plot(), b.plot());
    if side_contact(p, q) {
        return Some(((p.x - q.x).abs() - p.half_x - q.half_x).abs() < 0.02);
    }
    let (p, q) = (a.body(), b.body());
    let tolerance = if a.kind.is_wall() || b.kind.is_wall() {
        2.05
    } else {
        0.12
    };
    let xgap = (p.x - q.x).abs() - p.half_x - q.half_x;
    let zgap = (p.z - q.z).abs() - p.half_z - q.half_z;
    let ox = (p.x + p.half_x).min(q.x + q.half_x) - (p.x - p.half_x).max(q.x - q.half_x);
    let oz = (p.z + p.half_z).min(q.z + q.half_z) - (p.z - p.half_z).max(q.z - q.half_z);
    if xgap >= -0.16 && xgap <= tolerance && oz >= 0.6 {
        Some(true)
    } else if zgap >= -0.16 && zgap <= tolerance && ox >= 0.6 {
        Some(false)
    } else {
        None
    }
}
fn join_bounds(piece: Structure) -> Obstacle {
    if piece.kind.is_tower() {
        if let Some(p) = piece.kind.roof_profile() {
            return rect(
                (piece.x, piece.z),
                piece.rotation,
                (0., 0.),
                p.half_x,
                p.half_z,
            );
        }
    }
    piece.body()
}
pub fn joins(pieces: &[Structure]) -> Vec<FortJoin> {
    let mut result = Vec::new();
    for (i, a) in pieces.iter().enumerate() {
        if a.kind.is_wall() {
            continue;
        }
        for (j, b) in pieces.iter().enumerate() {
            if i == j
                || (!b.kind.is_wall() && (j < i || !a.kind.is_anchor() || !b.kind.is_anchor()))
            {
                continue;
            }
            let Some(horizontal) = connection_axis(*a, *b) else {
                continue;
            };
            let (p, q) = (join_bounds(*a), join_bounds(*b));
            let (along0, along1, mut across0, mut across1) = if horizontal {
                let (left, right) = if p.x < q.x { (p, q) } else { (q, p) };
                (
                    (p.z - p.half_z).max(q.z - q.half_z),
                    (p.z + p.half_z).min(q.z + q.half_z),
                    left.x + left.half_x - 0.16,
                    right.x - right.half_x + 0.16,
                )
            } else {
                let (low, high) = if p.z < q.z { (p, q) } else { (q, p) };
                (
                    (p.x - p.half_x).max(q.x - q.half_x),
                    (p.x + p.half_x).min(q.x + q.half_x),
                    low.z + low.half_z - 0.16,
                    high.z - high.half_z + 0.16,
                )
            };
            if along1 - along0 < 0.5 {
                continue;
            }
            if across1 - across0 < 0.1 {
                let mid = (across0 + across1) * 0.5;
                across0 = mid - 0.05;
                across1 = mid + 0.05;
            }
            let shape = if horizontal {
                Obstacle {
                    x: (across0 + across1) * 0.5,
                    z: (along0 + along1) * 0.5,
                    half_x: (across1 - across0) * 0.5,
                    half_z: (along1 - along0) * 0.5,
                }
            } else {
                Obstacle {
                    x: (along0 + along1) * 0.5,
                    z: (across0 + across1) * 0.5,
                    half_x: (along1 - along0) * 0.5,
                    half_z: (across1 - across0) * 0.5,
                }
            };
            let wooden = |k| {
                matches!(
                    k,
                    BuildingKind::WoodenWall
                        | BuildingKind::TowerWooden
                        | BuildingKind::GatehouseWooden
                )
            };
            result.push(FortJoin {
                a: a.id,
                b: b.id,
                shape,
                base: a.y.min(b.y),
                top: a.level().min(b.level()),
                wood: wooden(a.kind) && wooden(b.kind),
            });
        }
    }
    result
}
/// Only the facing parapet at a same-level tower transition is opened.
/// The floor and all exterior faces stay intact; removing a neighbour restores them.
pub fn tower_openings(piece: Structure, pieces: &[Structure]) -> Vec<ParapetBlock> {
    tower_openings_from(piece, pieces, &joins(pieces))
}
pub fn tower_openings_from(
    piece: Structure,
    pieces: &[Structure],
    connections: &[FortJoin],
) -> Vec<ParapetBlock> {
    if !piece.kind.is_tower() {
        return Vec::new();
    }
    connections
        .iter()
        .filter_map(|j| {
            let other_id = if j.a == piece.id {
                j.b
            } else if j.b == piece.id {
                j.a
            } else {
                return None;
            };
            let other = pieces.iter().find(|p| p.id == other_id)?;
            if !other.kind.is_tower() || (piece.level() - other.level()).abs() > 0.05 {
                return None;
            }
            let mut shape = j.shape;
            shape.half_x += 0.12;
            shape.half_z += 0.12;
            Some(ParapetBlock {
                shape,
                bottom: piece.level() + 0.035,
                top: piece.level().max(piece.y + piece.kind.height()) + 6.,
            })
        })
        .collect()
}
/// Exposed sides follow straight runs, corners and junctions, never the model's default rotation.
pub fn exposed_edges(piece: Structure, pieces: &[Structure]) -> Vec<Point> {
    [(1., 0.), (-1., 0.), (0., 1.), (0., -1.)]
        .into_iter()
        .filter(|d| {
            let point = (piece.x + d.0 * 2., piece.z + d.1 * 2.);
            !pieces.iter().any(|b| {
                b.id != piece.id
                    && b.owner == piece.owner
                    && b.level() >= piece.level() - 0.05
                    && covered_wall(point, *b)
            })
        })
        .collect()
}
#[derive(Clone, Copy)]
pub struct ParapetBlock {
    pub shape: Obstacle,
    pub bottom: f32,
    pub top: f32,
}
pub fn parapets(
    kind: BuildingKind,
    origin: Point,
    rotation: u8,
    edges: &[Point],
) -> Vec<ParapetBlock> {
    let Some(profile) = kind.roof_profile() else {
        return Vec::new();
    };
    let half = if kind.is_wall() {
        0.88
    } else {
        profile.half_x.min(profile.half_z) + 0.05
    };
    let mut out = Vec::new();
    for &(dx, dz) in edges {
        let side = |along: f32, _h: f32, w: f32| {
            rect(
                origin,
                rotation,
                (dx * half + dz * along, dz * half + dx * along),
                if dx != 0. { 0.14 } else { w },
                if dz != 0. { 0.14 } else { w },
            )
        };
        let n = (half * 2. / 0.75).ceil() as u32;
        for i in 0..n {
            let along = -half + (i as f32 + 0.5) * (half * 2. / n as f32);
            if kind.is_gate() && dz > 0. && along.abs() < 0.65 {
                continue;
            }
            out.push(ParapetBlock {
                shape: side(along, 0., half / n as f32),
                bottom: profile.height,
                top: profile.height + 0.5,
            });
            out.push(ParapetBlock {
                shape: side(along, 0., 0.22),
                bottom: profile.height + 0.5,
                top: profile.height + 1.2,
            });
        }
    }
    out
}
#[cfg(test)]
mod attachment_tests {
    use super::*;
    fn s(id: u64, kind: BuildingKind, x: f32, z: f32) -> Structure {
        Structure {
            id,
            kind,
            x,
            y: 0.,
            z,
            rotation: 0,
            owner: 0,
        }
    }
    #[test]
    fn wall_drag_ignores_anchor_interiors_and_replacement_is_owned() {
        let t = s(1, BuildingKind::TowerStairwell, 0., 0.);
        assert!(covered_wall((1., 1.), t));
        let mut w = s(2, BuildingKind::StoneWall, 1., 1.);
        assert!(replaced_wall(t, w));
        w.owner = 1;
        assert!(!replaced_wall(t, w));
    }
    #[test]
    fn lower_neighbours_keep_upper_parapets_and_small_gate_crenels_rotate() {
        let a = s(1, BuildingKind::StoneWallCrenellated, 1., 1.);
        let low = s(2, BuildingKind::StoneWallLow, 3., 1.);
        assert_eq!(exposed_edges(a, &[a, low]).len(), 4);
        let edges = [(1., 0.), (-1., 0.), (0., 1.), (0., -1.)];
        for rotation in 0..4 {
            let blocks = parapets(BuildingKind::GatehouseSmall, (20., 30.), rotation, &edges);
            assert!(blocks
                .iter()
                .any(|b| b.top > BuildingKind::GatehouseSmall.roof_profile().unwrap().height + 1.));
            let roof = crate::building_roof::BuildingRoof::new(
                BuildingKind::GatehouseSmall,
                20.,
                0.,
                30.,
                rotation,
            )
            .unwrap();
            assert!(!blocks
                .iter()
                .any(|b| b.shape.contains(roof.hatch(), crate::CLEARANCE)));
        }
    }
    #[test]
    fn tower_joint_seals_the_real_body_gap() {
        let a = s(1, BuildingKind::TowerRoundShort, 0., 0.);
        let b = s(2, BuildingKind::TowerRoundShort, a.plot().half_x * 2., 0.);
        assert!(may_join(a, b));
        let mut solids = vec![a.body(), b.body()];
        solids.extend(joins(&[a, b]).into_iter().map(|j| j.shape));
        let seam = a.plot().half_x;
        assert!(!crate::segment_clear((seam, -8.), (seam, 8.), &solids));
    }
    #[test]
    fn wall_ends_seal_the_remaining_grid_distance_to_ordinary_buildings() {
        let wall = s(1, BuildingKind::StoneWallLow, 1., 1.);
        let mut house = s(2, BuildingKind::Housing, 0., 1.);
        house.x = wall.x + wall.body().half_x + house.body().half_x + 1.1;
        let connections = joins(&[wall, house]);
        assert_eq!(connections.len(), 1);
        let joint = connections[0];
        assert_eq!(joint.top, wall.level());
        let mid = (wall.body().x + wall.body().half_x + house.body().x - house.body().half_x) * 0.5;
        assert!(!crate::segment_clear(
            (mid, -4.),
            (mid, 6.),
            &[wall.body(), house.body(), joint.shape]
        ));
        house.owner = 1;
        assert!(joins(&[wall, house]).is_empty());
        house.owner = 0;
        house.x += 2.;
        assert!(joins(&[wall, house]).is_empty());
    }
    #[test]
    fn round_towers_join_at_the_roof_and_open_only_the_shared_battlements() {
        for rotation in 0..4 {
            let mut a = s(1, BuildingKind::TowerRoundShort, 0., 0.);
            a.rotation = rotation;
            let b = s(
                2,
                a.kind,
                a.body().half_x + a.kind.obstacle(0., 0.).half_x + 0.0002,
                0.,
            );
            let connections = joins(&[a, b]);
            assert_eq!(connections.len(), 1);
            let floor = a.kind.roof_profile().unwrap();
            assert!(connections[0].shape.x - connections[0].shape.half_x < floor.half_x);
            let cuts = tower_openings(a, &[a, b]);
            assert_eq!(cuts.len(), 1);
            assert!(cuts[0].shape.contains((a.body().half_x - 0.2, 0.), 0.));
            assert!(!cuts[0].shape.contains((-a.body().half_x + 0.2, 0.), 0.));
            assert!(cuts[0].bottom > a.level());
            assert!(
                cuts[0].top > a.level() + 3.,
                "visual battlements extend beyond the collision model height"
            );
            assert!(tower_openings(a, &[a]).is_empty());
            let mut lower = b;
            lower.y = -2.;
            assert!(tower_openings(a, &[a, lower]).is_empty());
        }
    }
    #[test]
    fn crenels_follow_run_and_corner() {
        let a = s(1, BuildingKind::StoneWallCrenellated, 1., 1.);
        let east = s(2, a.kind, 3., 1.);
        let north = s(3, a.kind, 1., 3.);
        let edges = exposed_edges(a, &[a, east, north]);
        assert_eq!(edges, vec![(-1., 0.), (0., -1.)]);
        let shapes = parapets(a.kind, (a.x, a.z), 0, &edges);
        assert!(!shapes.is_empty());
        assert!(shapes.iter().all(|b| b.shape.x < 1. || b.shape.z < 1.));
    }
}
