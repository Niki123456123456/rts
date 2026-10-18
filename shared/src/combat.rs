//! Shared melee distances and connected enemy formations.
use crate::{distance, Point};
pub const SENSE_RADIUS: f32 = 8.;
pub const PURSUIT_RADIUS: f32 = 16.;
pub const GROUP_RADIUS: f32 = 3.;
pub const MELEE_RANGE: f32 = 1.05;
pub const APPROACH_RADIUS: f32 = 0.9;
pub const SCAN_SECONDS: f32 = 0.25;
#[derive(Clone, Copy)]
pub struct GroupMember {
    pub id: u64,
    pub side: u32,
    pub point: Point,
    pub height: f32,
}
pub fn enemy_group(clicked: u64, units: &[GroupMember]) -> Vec<u64> {
    let Some(first) = units.iter().find(|u| u.id == clicked) else {
        return Vec::new();
    };
    let mut found = vec![first.id];
    let mut index = 0;
    while index < found.len() {
        let at = units.iter().find(|u| u.id == found[index]).unwrap();
        for u in units.iter().filter(|u| u.side == first.side) {
            if !found.contains(&u.id)
                && distance(at.point, u.point).hypot(at.height - u.height) <= GROUP_RADIUS
            {
                found.push(u.id);
            }
        }
        index += 1;
    }
    found
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicked_group_is_connected_and_does_not_include_other_sides_or_distant_units() {
        let units = [
            (1, 0, (0., 0.)),
            (2, 0, (2., 0.)),
            (3, 0, (4., 0.)),
            (4, 0, (12., 0.)),
            (5, 1, (1., 0.)),
        ]
        .map(|(id, side, point)| GroupMember {
            id,
            side,
            point,
            height: 0.,
        });
        assert_eq!(enemy_group(1, &units), vec![1, 2, 3]);
        assert_eq!(enemy_group(4, &units), vec![4]);
        assert!(enemy_group(99, &units).is_empty());
        let mut elevated = units.to_vec();
        elevated.push(GroupMember {
            id: 6,
            side: 0,
            point: (0., 0.),
            height: 8.,
        });
        assert_eq!(enemy_group(1, &elevated), vec![1, 2, 3]);
    }
}
