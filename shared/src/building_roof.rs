//! Shared walkable roofs; model-specific geometry in centred, unrotated metres.
use crate::{distance, shortest_path, BuildingKind, Obstacle, Point};

pub const UNIT_CENTER_HEIGHT: f32 = 0.75;
pub const ROOF_UNIT_SPACING: f32 = 0.7;
// Retain the first eight slot IDs so existing hosted reservations stay valid.
const LEGACY_SLOTS: [Point; 25] = [
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
pub type Position = (f32, f32, f32);

#[derive(Clone, Copy, Debug)]
pub struct RoofProfile {
    pub height: f32,
    pub half_x: f32,
    pub half_z: f32,
    pub hatch: Point,
    pub doors: &'static [Point],
}
impl RoofProfile {
    /// Reserve unit-radius clearance at both roof edges; use the same pitch everywhere.
    pub fn dimensions(self) -> (usize, usize) {
        let cells = |half: f32| {
            ((2.0 * (half - crate::CLEARANCE) / ROOF_UNIT_SPACING + 0.00001).floor() as usize) + 1
        };
        (cells(self.half_x), cells(self.half_z))
    }
    pub fn capacity(self) -> usize {
        let (width, depth) = self.dimensions();
        width * depth
    }
    fn slot(self, slot: usize) -> Point {
        let (width, depth) = self.dimensions();
        assert!(slot < width * depth);
        if (width, depth) == (5, 5) {
            return LEGACY_SLOTS[slot];
        }
        (
            (slot % width) as f32 * ROOF_UNIT_SPACING
                - (width - 1) as f32 * ROOF_UNIT_SPACING * 0.5,
            (slot / width) as f32 * ROOF_UNIT_SPACING
                - (depth - 1) as f32 * ROOF_UNIT_SPACING * 0.5,
        )
    }
}
impl BuildingKind {
    /// Original meshes stay unchanged. Add profiles here to enable more roofs.
    pub fn roof_profile(self) -> Option<RoofProfile> {
        Some(match self {
            Self::StoneWall => RoofProfile {
                height: 6.663,
                half_x: 1.0,
                half_z: 1.0,
                hatch: (0., 0.),
                doors: &[],
            },
            Self::TowerStairwell => RoofProfile {
                height: 6.663,
                half_x: 1.98,
                half_z: 1.98,
                hatch: (-0.75, 0.9),
                doors: &[(-2.7, 1.04), (-0.99, 2.7)],
            },
            Self::TowerLookout => RoofProfile {
                height: 15.513,
                half_x: 1.78,
                half_z: 1.78,
                hatch: (-0.782, 0.85),
                doors: &[(-2.9, 0.968), (-1.055, 2.9)],
            },
            Self::TowerSquare => RoofProfile {
                height: 14.399,
                half_x: 3.02,
                half_z: 3.02,
                hatch: (0.274, 1.796),
                doors: &[(0.02, 4.1)],
            },
            _ => return None,
        })
    }
}

#[derive(Clone, Copy)]
pub struct BuildingRoof {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: u8,
    pub profile: RoofProfile,
}
impl BuildingRoof {
    pub fn new(kind: BuildingKind, x: f32, y: f32, z: f32, rotation: u8) -> Option<Self> {
        Some(Self {
            x,
            y,
            z,
            rotation,
            profile: kind.roof_profile()?,
        })
    }
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
        BuildingRoof {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            rotation: (4 - self.rotation % 4) % 4,
            profile: self.profile,
        }
        .world((p.0 - self.x, p.1 - self.z))
    }
    pub fn roof_y(self) -> f32 {
        self.y + self.profile.height
    }
    pub fn roof_contains(self, p: Point) -> bool {
        let (x, z) = self.local(p);
        x.abs() <= self.profile.half_x && z.abs() <= self.profile.half_z
    }
    pub fn capacity(self) -> usize {
        self.profile.capacity()
    }
    pub fn slot(self, slot: usize) -> Point {
        self.world(self.profile.slot(slot))
    }
    pub fn door_count(self) -> u8 {
        self.profile.doors.len() as u8
    }
    pub fn door(self, door: u8) -> Point {
        self.world(self.profile.doors[door as usize])
    }
    pub fn hatch(self) -> Point {
        self.world(self.profile.hatch)
    }
    pub fn roof_obstacles(self) -> Vec<Obstacle> {
        // The unchanged closed mesh supports the full roof, including the hatch.
        let mut shapes = Vec::new();
        for (x, z, hx, hz) in [
            (self.profile.half_x + 5., 0., 5., 10.),
            (-self.profile.half_x - 5., 0., 5., 10.),
            (0., self.profile.half_z + 5., 10., 5.),
            (0., -self.profile.half_z - 5., 10., 5.),
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
        let mut slots = (0..self.capacity()).collect::<Vec<_>>();
        slots.sort_by(|a, b| {
            distance(self.slot(*a), point).total_cmp(&distance(self.slot(*b), point))
        });
        slots
    }
}

pub fn supports_roof(kind: &str) -> bool {
    BuildingKind::from_name(kind)
        .and_then(BuildingKind::roof_profile)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{free, UNIT_SPACING};
    #[test]
    fn rotated_doors_hatch_and_roof_slots_are_reachable_and_separated() {
        assert_eq!(LEGACY_SLOTS.len(), 25);
        for x in [-1.4, -0.7, 0., 0.7, 1.4] {
            for z in [-1.4, -0.7, 0., 0.7, 1.4] {
                assert_eq!(LEGACY_SLOTS.iter().filter(|p| **p == (x, z)).count(), 1);
            }
        }
        for kind in [
            BuildingKind::TowerStairwell,
            BuildingKind::TowerLookout,
            BuildingKind::TowerSquare,
            BuildingKind::StoneWall,
        ] {
            for rotation in 0..4 {
                let t = BuildingRoof::new(kind, 100., 3., 100., rotation).unwrap();
                let obstacles = t.roof_obstacles();
                assert_eq!(
                    t.profile.dimensions(),
                    match kind {
                        BuildingKind::TowerSquare => (8, 8),
                        BuildingKind::StoneWall => (3, 3),
                        _ => (5, 5),
                    }
                );
                assert!(
                    t.capacity() <= 256,
                    "Persisted roof slot IDs must fit in u8"
                );
                for i in 0..t.capacity() {
                    let p = t.slot(i);
                    assert!(
                        t.roof_contains(p) && free(p, &obstacles),
                        "{kind:?}: slot {i}"
                    );
                    let path = t.roof_path(t.hatch(), p).unwrap();
                    assert!(path
                        .iter()
                        .all(|p| (p.1 - t.roof_y() - UNIT_CENTER_HEIGHT).abs() < 0.001));
                    assert!(t.roof_path(p, t.hatch()).is_some());
                    let mut nearest = f32::INFINITY;
                    for j in 0..t.capacity() {
                        if i != j {
                            nearest = nearest.min(distance(p, t.slot(j)));
                        }
                        if j < i {
                            assert!(distance(p, t.slot(j)) >= UNIT_SPACING);
                        }
                    }
                    assert!(
                        (nearest - ROOF_UNIT_SPACING).abs() < 0.0001,
                        "{kind:?}: spacing must be identical on every roof"
                    );
                }
                for door in 0..t.door_count() {
                    assert!(
                        free(t.door(door), &[kind.rotated_obstacle(t.x, t.z, rotation)]),
                        "{kind:?}: door {door}"
                    );
                }
                let p = (-1.1, 0.4);
                assert!(distance(t.local(t.world(p)), p) < 0.0001);
            }
        }
        for kind in BuildingKind::ALL {
            assert_eq!(
                kind.roof_profile().is_some(),
                matches!(
                    kind,
                    BuildingKind::TowerStairwell
                        | BuildingKind::TowerLookout
                        | BuildingKind::TowerSquare
                        | BuildingKind::StoneWall
                )
            );
        }
    }
}
