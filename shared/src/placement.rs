//! Magnetic edge snapping uses the actual visible building bounds, without spacer plots.
use crate::{BuildingKind, Point};
pub fn snap(
    kind: BuildingKind,
    at: Point,
    rotation: u8,
    neighbours: &[(BuildingKind, f32, f32, u8)],
) -> Point {
    let grid = kind.rotated_snap_position(at.0, at.1, rotation);
    if kind.is_wall() {
        return grid;
    }
    let own = kind.construction_bounds(0., 0., rotation);
    let mut xs = vec![grid.0];
    let mut zs = vec![grid.1];
    for &(k, x, z, r) in neighbours {
        let b = k.construction_bounds(x, z, r);
        if (grid.1 - z).abs() < own.half_z + b.half_z - 0.1 {
            for side in [-1., 1.] {
                let v = x + side * (own.half_x + b.half_x + 0.0002);
                if (v - at.0).abs() <= 1.2 {
                    xs.push(v);
                }
            }
        }
        if (grid.0 - x).abs() < own.half_x + b.half_x - 0.1 {
            for side in [-1., 1.] {
                let v = z + side * (own.half_z + b.half_z + 0.0002);
                if (v - at.1).abs() <= 1.2 {
                    zs.push(v);
                }
            }
        }
    }
    let mut candidates = Vec::new();
    for (i, &x) in xs.iter().enumerate() {
        for (j, &z) in zs.iter().enumerate() {
            if i == 0 && j == 0 {
                continue;
            }
            let a = kind.construction_bounds(x, z, rotation);
            if neighbours
                .iter()
                .all(|&(k, bx, bz, r)| !a.overlaps(k.construction_bounds(bx, bz, r)))
            {
                candidates.push((x, z));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| crate::distance(*a, at).total_cmp(&crate::distance(*b, at)))
        .unwrap_or(grid)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buildings_touch_without_overlapping_and_snap_is_idempotent() {
        for kind in [
            BuildingKind::Housing,
            BuildingKind::Granary,
            BuildingKind::Barracks,
            BuildingKind::Market,
        ] {
            for r in 0..4 {
                let other = BuildingKind::Housing.construction_bounds(20., 20., 1);
                let mine = kind.construction_bounds(0., 0., r);
                let input = (20. + other.half_x + mine.half_x, 20.);
                let neighbours = [(BuildingKind::Housing, 20., 20., 1)];
                let p = snap(kind, input, r, &neighbours);
                assert!((p.0 - input.0).abs() < 0.001);
                assert!(!kind.construction_bounds(p.0, p.1, r).overlaps(other));
                assert_eq!(snap(kind, p, r, &neighbours), p);
            }
        }
    }
}
