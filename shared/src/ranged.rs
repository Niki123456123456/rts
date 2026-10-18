//! Bow range and projectile geometry shared by the simulation and renderer.
use crate::{distance, Obstacle, Point};
pub const BASE_RANGE: f32 = 14.;
pub const HEIGHT_BONUS: f32 = 0.6;
pub const SHOT_INTERVAL: f32 = 1.25;
pub const SHOT_DAMAGE: f32 = 0.18;
pub const ARROW_SPEED: f32 = 22.;
pub type Position = (f32, f32, f32);
pub fn bow_range(unit_y: f32) -> f32 {
    BASE_RANGE + (unit_y - 0.75).max(0.) * HEIGHT_BONUS
}
pub fn arrow_point(start: Position, end: Position, progress: f32) -> Position {
    let t = progress.clamp(0., 1.);
    let arc = (distance((start.0, start.2), (end.0, end.2)) * 0.2 + 0.8).min(7.);
    (
        start.0 + (end.0 - start.0) * t,
        start.1 + (end.1 - start.1) * t + 4. * arc * t * (1. - t),
        start.2 + (end.2 - start.2) * t,
    )
}
#[derive(Clone, Copy)]
pub struct ShotBlock {
    pub id: u64,
    pub shape: Obstacle,
    pub base: f32,
    pub top: f32,
}
pub fn shot_clear(
    start: Position,
    end: Position,
    blocks: &[ShotBlock],
    ignored: Option<u64>,
    terrain: impl Fn(Point) -> f32,
) -> bool {
    let length = distance((start.0, start.2), (end.0, end.2)).hypot(end.1 - start.1);
    let steps = (length / 0.2).ceil().max(2.) as usize;
    (1..steps).all(|i| {
        let p = arrow_point(start, end, i as f32 / steps as f32);
        p.1 > terrain((p.0, p.2)) + 0.02
            && !blocks.iter().any(|b| {
                Some(b.id) != ignored
                    && p.1 >= b.base
                    && p.1 < b.top
                    && b.shape.contains((p.0, p.2), 0.)
            })
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn height_adds_range_and_arrows_need_a_clear_arc() {
        assert_eq!(bow_range(0.75), BASE_RANGE);
        assert_eq!(bow_range(7.413), BASE_RANGE + 6.663 * HEIGHT_BONUS);
        assert_eq!(bow_range(5.75), BASE_RANGE + 5. * HEIGHT_BONUS);
        let a = (0., 1.25, 0.);
        let b = (12., 0.9, 0.);
        assert_eq!(arrow_point(a, b, 0.), a);
        assert_eq!(arrow_point(a, b, 1.), b);
        assert!(arrow_point(a, b, 0.5).1 > 3.);
        let block = ShotBlock {
            id: 1,
            shape: Obstacle {
                x: 6.,
                z: 0.,
                half_x: 1.,
                half_z: 1.,
            },
            base: 0.,
            top: 8.,
        };
        assert!(!shot_clear(a, b, &[block], None, |_| 0.));
        assert!(shot_clear(a, b, &[block], Some(1), |_| 0.));
        assert!(shot_clear(
            (0., 9., 0.),
            (12., 9., 0.),
            &[block],
            None,
            |_| 0.
        ));
    }
}
