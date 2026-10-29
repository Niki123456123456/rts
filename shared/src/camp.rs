//! Collision-safe camp positions shared by waiting visuals and worker assignment.
use crate::{
    building_roof::BuildingRoof, distance, free, shortest_path, Obstacle, Point, UNIT_SPACING,
};
pub fn spots(keep: BuildingRoof, obstacles: &[Obstacle], count: usize) -> Vec<Point> {
    let mut result = Vec::new();
    for i in 0..128 {
        let angle = 2.1 + i as f32 * 2.39996;
        let radius = 1.9 + (i / 12) as f32 * 0.65;
        let point = keep.world((radius * angle.cos(), 11. + radius * angle.sin()));
        if free(point, obstacles) && result.iter().all(|p| distance(point, *p) >= UNIT_SPACING) {
            result.push(point);
            if result.len() >= count {
                break;
            }
        }
    }
    result
}
pub fn arrival(
    keep: BuildingRoof,
    goal: Point,
    obstacles: &[Obstacle],
) -> Option<(Point, Vec<Point>)> {
    [(-8.5, 12.5), (-9., 11.), (9., 14.), (0., 15.)]
        .into_iter()
        .find_map(|p| {
            let start = keep.world(p);
            shortest_path(start, goal, obstacles).map(|path| (start, path))
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{segment_clear, BuildingKind};
    #[test]
    fn waiting_and_arriving_settlers_never_cross_keep_or_neighbouring_buildings() {
        for rotation in 0..4 {
            let keep = BuildingRoof::new(BuildingKind::Keep, 0., 0., 0., rotation).unwrap();
            let neighbour = keep.world((-6., 12.));
            let blocked = vec![
                BuildingKind::Keep.rotated_obstacle(0., 0., rotation),
                Obstacle {
                    x: neighbour.0,
                    z: neighbour.1,
                    half_x: 1.,
                    half_z: 1.,
                },
            ];
            let points = spots(keep, &blocked, 16);
            assert_eq!(points.len(), 16);
            for goal in points {
                assert!(free(goal, &blocked));
                let (mut previous, path) =
                    arrival(keep, goal, &blocked).expect("camp must be reachable");
                for next in path {
                    assert!(segment_clear(previous, next, &blocked));
                    previous = next;
                }
                assert!(distance(previous, goal) < 0.001);
            }
        }
    }
}
