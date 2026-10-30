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
