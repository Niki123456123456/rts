//! Walkable stair-tower geometry in metres, relative to its centred, unrotated model.
use crate::{distance, shortest_path, BuildingKind, Obstacle, Point};

pub const ROOF_HEIGHT: f32 = 6.663;
pub const UNIT_CENTER_HEIGHT: f32 = 0.75;
pub const ROOF_SIDE: usize = 5;
pub const ROOF_CAPACITY: usize = ROOF_SIDE * ROOF_SIDE;
// Retain the first eight slot IDs so existing hosted reservations stay valid.
pub const ROOF_SLOTS: [Point; ROOF_CAPACITY] = [
    (-1.4, -1.4),
    (-0.7, -1.4),
    (0., -1.4),
    (0.7, -1.4),
    (1.4, -1.4),
    (1.4, 0.),
    (1.4, 0.7),
    (1.4, 1.4),
    (-1.4, -0.7),
    (-0.7, -0.7),
    (0., -0.7),
    (0.7, -0.7),
    (1.4, -0.7),
    (-1.4, 0.),
    (-0.7, 0.),
    (0., 0.),
    (0.7, 0.),
    (-1.4, 0.7),
    (-0.7, 0.7),
    (0., 0.7),
    (0.7, 0.7),
    (-1.4, 1.4),
    (-0.7, 1.4),
    (0., 1.4),
    (0.7, 1.4),
];
pub const HATCH_LANDING: Point = (-0.75, 0.9);
pub type Position = (f32, f32, f32);

#[derive(Clone, Copy)]
pub struct StairTower {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: u8,
}
impl StairTower {
    pub fn world(self, p: Point) -> Point {
        let (x, z) = match self.rotation % 4 {
            0 => p,
            1 => (p.1, -p.0),
            2 => (-p.0, -p.1),
            _ => (-p.1, p.0),
        };
        (self.x + x, self.z + z)
    }
    pub fn local(self, p: Point) -> Point {
        StairTower {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            rotation: (4 - self.rotation % 4) % 4,
        }
        .world((p.0 - self.x, p.1 - self.z))
    }
    pub fn roof_y(self) -> f32 {
        self.y + ROOF_HEIGHT
    }
    pub fn roof_contains(self, p: Point) -> bool {
        let (x, z) = self.local(p);
        x.abs() <= 1.98 && z.abs() <= 1.98
    }
    pub fn slot(self, slot: usize) -> Point {
        self.world(ROOF_SLOTS[slot])
    }
    pub fn door(self, door: u8) -> Point {
        self.world(if door == 0 {
            (-2.7, 1.04)
        } else {
            (-0.99, 2.7)
        })
    }
    pub fn roof_obstacles(self) -> Vec<Obstacle> {
        // The unchanged closed mesh supports the full roof, including the hatch.
        let mut shapes = Vec::new();
        for (x, z, hx, hz) in [
            (6.98, 0., 5., 10.),
            (-6.98, 0., 5., 10.),
            (0., 6.98, 10., 5.),
            (0., -6.98, 10., 5.),
        ] {
            shapes.push(Obstacle {
                x,
                z,
                half_x: hx,
                half_z: hz,
            });
        }
        shapes
            .into_iter()
            .map(|o| {
                let p = self.world((o.x, o.z));
                let (hx, hz) = if self.rotation % 2 == 0 {
                    (o.half_x, o.half_z)
                } else {
                    (o.half_z, o.half_x)
                };
                Obstacle {
                    x: p.0,
                    z: p.1,
                    half_x: hx,
                    half_z: hz,
                }
            })
            .collect()
    }
    pub fn roof_path(self, start: Point, goal: Point) -> Option<Vec<Position>> {
        shortest_path(start, goal, &self.roof_obstacles()).map(|path| {
            path.into_iter()
                .map(|p| (p.0, self.roof_y() + UNIT_CENTER_HEIGHT, p.1))
                .collect()
        })
    }
    pub fn sorted_slots(self, point: Point) -> Vec<usize> {
        let mut slots = (0..ROOF_SLOTS.len()).collect::<Vec<_>>();
        slots.sort_by(|a, b| {
            distance(self.slot(*a), point).total_cmp(&distance(self.slot(*b), point))
        });
        slots
    }
}

pub fn is_stair_tower(kind: &str) -> bool {
    BuildingKind::from_name(kind) == Some(BuildingKind::TowerStairwell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{free, UNIT_SPACING};
    #[test]
    fn rotated_doors_hatch_and_roof_slots_are_reachable_and_separated() {
        assert_eq!(ROOF_SLOTS.len(), 25);
        for x in [-1.4, -0.7, 0., 0.7, 1.4] {
            for z in [-1.4, -0.7, 0., 0.7, 1.4] {
                assert_eq!(ROOF_SLOTS.iter().filter(|p| **p == (x, z)).count(), 1);
            }
        }
        for rotation in 0..4 {
            let t = StairTower {
                x: 100.,
                y: 3.,
                z: 100.,
                rotation,
            };
            let obstacles = t.roof_obstacles();
            for (i, _) in ROOF_SLOTS.iter().enumerate() {
                let p = t.slot(i);
                assert!(t.roof_contains(p) && free(p, &obstacles));
                let path = t.roof_path(t.world(HATCH_LANDING), p).unwrap();
                assert!(path.iter().all(|p| (p.1 - 10.413).abs() < 0.001));
                assert!(t.roof_path(p, t.world(HATCH_LANDING)).is_some());
                for j in 0..i {
                    assert!(distance(p, t.slot(j)) >= UNIT_SPACING);
                }
            }
            for door in 0..2 {
                assert!(free(
                    t.door(door),
                    &[BuildingKind::TowerStairwell.rotated_obstacle(t.x, t.z, rotation)]
                ));
            }
            let p = (-1.1, 0.4);
            assert!(distance(t.local(t.world(p)), p) < 0.0001);
        }
    }
}
