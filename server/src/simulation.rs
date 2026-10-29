pub(crate) fn step_towards(x: f32, z: f32, target_x: f32, target_z: f32, step: f32) -> (f32, f32) {
    let (dx, dz) = (target_x - x, target_z - z);
    let distance = dx.hypot(dz);
    if distance <= step {
        (target_x, target_z)
    } else {
        (x + dx / distance * step, z + dz / distance * step)
    }
}

// Client mesh, construction and simulation use the same interpolated surface.
pub(crate) fn terrain_height(x: f32, z: f32) -> f32 {
    stronghold_map::terrain::height(x, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_has_fixed_speed_and_never_overshoots() {
        let (x, z) = step_towards(0.0, 0.0, 3.0, 4.0, 0.15);
        assert!((x.hypot(z) - 0.15).abs() < 0.00001);
        assert_eq!(step_towards(x, z, 3.0, 4.0, 10.0), (3.0, 4.0));
        assert_eq!(step_towards(3.0, 4.0, 3.0, 4.0, 0.15), (3.0, 4.0));
    }

    #[test]
    fn terrain_matches_shared_landscape() {
        for (x, z) in [(0.0, 0.0), (-195.0, -275.0), (92.0, 0.0), (-115.0, 55.0)] {
            assert_eq!(terrain_height(x, z), stronghold_map::terrain::height(x, z));
        }
    }
}

use stronghold_map::{self as map, Obstacle, Point};

/// Minimum total squared travel distance keeps formation assignments from crossing.
/// Hungarian matching, with stable input order to break ties deterministically.
pub(crate) fn formation_assignment(units: &[Point], slots: &[Point]) -> Vec<usize> {
    let n = units.len();
    assert_eq!(n, slots.len());
    let mut unit_potential = vec![0.0; n + 1];
    let mut slot_potential = vec![0.0; n + 1];
    let mut matched = vec![0; n + 1];
    let mut previous = vec![0; n + 1];
    for unit in 1..=n {
        matched[0] = unit;
        let mut slot = 0;
        let mut minimum = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[slot] = true;
            let i = matched[slot];
            let mut delta = f64::INFINITY;
            let mut next = 0;
            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let dx = units[i - 1].0 as f64 - slots[j - 1].0 as f64;
                let dz = units[i - 1].1 as f64 - slots[j - 1].1 as f64;
                let cost = dx * dx + dz * dz - unit_potential[i] - slot_potential[j];
                if cost < minimum[j] {
                    minimum[j] = cost;
                    previous[j] = slot;
                }
                if minimum[j] < delta {
                    delta = minimum[j];
                    next = j;
                }
            }
            for j in 0..=n {
                if used[j] {
                    unit_potential[matched[j]] += delta;
                    slot_potential[j] -= delta;
                } else {
                    minimum[j] -= delta;
                }
            }
            slot = next;
            if matched[slot] == 0 {
                break;
            }
        }
        loop {
            let prev = previous[slot];
            matched[slot] = matched[prev];
            slot = prev;
            if slot == 0 {
                break;
            }
        }
    }
    let mut result = vec![0; n];
    for j in 1..=n {
        result[matched[j] - 1] = j - 1;
    }
    result
}

pub(crate) fn can_skip_corner(
    start: Point,
    corner: Point,
    next: Point,
    obstacles: &[Obstacle],
) -> bool {
    map::distance(start, corner) < 1.2 && map::segment_clear(start, next, obstacles)
}

fn segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let delta = (b.0 - a.0, b.1 - a.1);
    let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
    let t = if length_squared > 0.0 {
        (((p.0 - a.0) * delta.0 + (p.1 - a.1) * delta.1) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    map::distance(p, (a.0 + delta.0 * t, a.1 + delta.1 * t))
}

fn crowd_waypoint(start: Point, goal: Point, peers: &[Point], obstacles: &[Obstacle]) -> Point {
    if peers
        .iter()
        .all(|p| segment_distance(*p, start, goal) >= map::UNIT_SPACING)
    {
        return goal;
    }
    // Local visibility routing avoids walking into the middle of an idle army.
    // The swept-circle check below still tests every unit, including distant ones.
    let mut nearby = peers
        .iter()
        .copied()
        .filter(|p| map::distance(start, *p) >= map::UNIT_SPACING - 0.00001)
        .collect::<Vec<_>>();
    nearby.sort_by(|a, b| map::distance(start, *a).total_cmp(&map::distance(start, *b)));
    nearby.truncate(12);
    if nearby
        .iter()
        .any(|p| map::distance(goal, *p) < map::UNIT_SPACING)
    {
        return goal;
    }
    let available = |p: Point| {
        map::free(p, obstacles)
            && nearby
                .iter()
                .all(|q| map::distance(p, *q) >= map::UNIT_SPACING)
    };
    let mut nodes = vec![start, goal];
    for p in &nearby {
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::TAU / 8.0;
            let vertex = (p.0 + angle.cos() * 0.62, p.1 + angle.sin() * 0.62);
            if available(vertex) {
                nodes.push(vertex);
            }
        }
    }
    for b in obstacles {
        for x in [
            b.x - b.half_x - map::CLEARANCE - 0.001,
            b.x + b.half_x + map::CLEARANCE + 0.001,
        ] {
            for z in [
                b.z - b.half_z - map::CLEARANCE - 0.001,
                b.z + b.half_z + map::CLEARANCE + 0.001,
            ] {
                if available((x, z)) {
                    nodes.push((x, z));
                }
            }
        }
    }
    map::visibility_path(&nodes, |a, b| {
        map::segment_clear(a, b, obstacles)
            && nearby
                .iter()
                .all(|p| segment_distance(*p, a, b) >= map::UNIT_SPACING - 0.00001)
    })
    .and_then(|p| p.first().copied())
    .unwrap_or(goal)
}

/// Swept-circle avoidance against the latest authoritative positions. Side steps
/// allow passing stationary units. Pre-upgrade overlaps may only move apart.
pub(crate) fn move_separated(
    start: Point,
    goal: Point,
    step: f32,
    peers: &[Point],
    obstacles: &[Obstacle],
) -> Point {
    let goal = crowd_waypoint(start, goal, peers, obstacles);
    let desired = step_towards(start.0, start.1, goal.0, goal.1, step);
    let dx = desired.0 - start.0;
    let dz = desired.1 - start.1;
    if dx.hypot(dz) < 0.00001 {
        return start;
    }
    let mut best = None;
    for degrees in [
        0.0_f32, 30.0, -30.0, 60.0, -60.0, 90.0, -90.0, 120.0, -120.0, 150.0, -150.0, 180.0,
    ] {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let candidate = (start.0 + dx * cos - dz * sin, start.1 + dx * sin + dz * cos);
        if !map::free(candidate, obstacles) || !map::segment_clear(start, candidate, obstacles) {
            continue;
        }
        if peers.iter().any(|p| {
            let before = map::distance(start, *p);
            if before < map::UNIT_SPACING - 0.00001 {
                map::distance(candidate, *p) <= before + 0.00001
            } else {
                segment_distance(*p, start, candidate) < map::UNIT_SPACING - 0.00001
            }
        }) {
            continue;
        }
        let score = map::distance(candidate, goal) + degrees.abs() * 0.00001;
        if best.is_none_or(|(_, cost)| score < cost) {
            best = Some((candidate, score));
        }
    }
    best.map(|(point, _)| point).unwrap_or(start)
}

fn attack_route(
    start: Point,
    slot: &map::AttackPosition,
    obstacles: &[Obstacle],
) -> Option<Vec<Point>> {
    if map::distance(start, slot.point) < 0.05 {
        return Some(vec![slot.point]);
    }
    // Travel outside the occupied melee perimeter, then approach the assigned
    // position perpendicular to its wall. Units on a wall cannot block access
    // to the other sides of the building.
    let outward = match slot.side {
        0 => (-0.9, 0.0),
        1 => (0.9, 0.0),
        2 => (0.0, -0.9),
        _ => (0.0, 0.9),
    };
    let staging = (slot.point.0 + outward.0, slot.point.1 + outward.1);
    if map::segment_clear(staging, slot.point, obstacles) {
        if let Some(mut path) = map::shortest_path_with_clearance(start, staging, obstacles, 1.2)
            .or_else(|| map::shortest_path(start, staging, obstacles))
        {
            path.push(slot.point);
            return Some(path);
        }
    }
    map::shortest_path(start, slot.point, obstacles)
}

/// Unique attack destinations balanced across reachable sides. Keep occupied
/// inner-ring assignments stable; promote queued units when space becomes free.
pub(crate) fn plan_attack(
    units: &[(u64, Point, Option<Point>)],
    target: Obstacle,
    obstacles: &[Obstacle],
    reserved: &[Point],
    occupied: &[Point],
) -> Option<Vec<(u64, Vec<Point>)>> {
    let positions = map::attack_positions(target);
    let mut reserved = reserved.to_vec();
    let mut sides = [0_usize; 4];
    for point in &reserved {
        if let Some(slot) = positions
            .iter()
            .find(|s| map::distance(s.point, *point) < 0.01)
        {
            sides[slot.side] += 1;
        }
    }
    let mut plans = Vec::new();
    for (id, start, previous) in units {
        let mut candidates = positions
            .iter()
            .filter(|s| {
                map::free(s.point, obstacles)
                    && !reserved
                        .iter()
                        .chain(occupied)
                        .any(|p| map::distance(s.point, *p) < map::UNIT_SPACING)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| {
            let keep = |slot: &map::AttackPosition| {
                slot.ring == 0 && previous.is_some_and(|p| map::distance(p, slot.point) < 0.01)
            };
            (!keep(a), a.ring, sides[a.side])
                .cmp(&(!keep(b), b.ring, sides[b.side]))
                .then_with(|| {
                    map::distance(*start, a.point).total_cmp(&map::distance(*start, b.point))
                })
        });
        let (slot, path) = candidates
            .into_iter()
            .find_map(|s| attack_route(*start, s, obstacles).map(|path| (s, path)))?;
        reserved.push(slot.point);
        sides[slot.side] += 1;
        plans.push((*id, path));
    }
    Some(plans)
}

#[cfg(test)]
mod crowd_tests {
    use super::*;
    #[test]
    fn formation_matching_preserves_front_to_back_order() {
        let units = [(0.0, 0.0), (0.0, 1.0)];
        let slots = [(0.0, 5.0), (0.0, 6.0)];
        assert_eq!(formation_assignment(&units, &slots), vec![0, 1]);
        assert!(formation_assignment(&[], &[]).is_empty());
    }

    #[test]
    fn army_reaches_formation_rows_without_collisions() {
        for end in [(0.0, 5.0), (-2.0, 7.0), (0.0, 7.0)] {
            let mut positions = (0..20)
                .map(|i| (-1.4 + (i % 5) as f32 * 0.7, -1.05 + (i / 5) as f32 * 0.7))
                .collect::<Vec<_>>();
            let slots = map::formation_positions((-2.0, 5.0), end, positions.len());
            let assignment = formation_assignment(&positions, &slots);
            for _ in 0..600 {
                for i in 0..positions.len() {
                    let goal = slots[assignment[i]];
                    if map::distance(positions[i], goal) < 0.00001 {
                        continue;
                    }
                    let peers = positions
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, p)| *p)
                        .collect::<Vec<_>>();
                    positions[i] = move_separated(positions[i], goal, 0.15, &peers, &[]);
                    for peer in peers {
                        assert!(map::distance(positions[i], peer) >= map::UNIT_SPACING - 0.00002);
                    }
                }
            }
            for (i, p) in positions.iter().enumerate() {
                assert!(
                    map::distance(*p, slots[assignment[i]]) < 0.0001,
                    "unit {i} stopped at {p:?}, wanted {:?}, drag end {end:?}",
                    slots[assignment[i]]
                );
            }
        }
    }

    #[test]
    fn attackers_spread_over_four_sides_and_keep_reserved_slots() {
        let target = map::BuildingKind::Granary.obstacle(0.0, 9.0);
        let units = (0..20)
            .map(|i| (i, ((i % 5) as f32 * 0.7, (i / 5) as f32 * 0.7), None))
            .collect::<Vec<_>>();
        let plans = plan_attack(&units, target, &[target], &[], &[]).unwrap();
        let goals = plans
            .iter()
            .map(|(_, p)| *p.last().unwrap())
            .collect::<Vec<_>>();
        for (i, p) in goals.iter().enumerate() {
            assert!(map::distance(*p, target.closest(*p)) <= map::ATTACK_RANGE);
            for q in &goals[i + 1..] {
                assert!(map::distance(*p, *q) >= map::UNIT_SPACING);
            }
        }
        let mut sides = [0; 4];
        for goal in &goals {
            let slot = map::attack_positions(target)
                .into_iter()
                .find(|s| map::distance(*goal, s.point) < 0.01)
                .unwrap();
            sides[slot.side] += 1;
        }
        assert_eq!(sides, [5, 5, 5, 5]);
        let more = plan_attack(&[(25, (-9.0, 9.0), None)], target, &[target], &goals, &[]).unwrap();
        assert!(goals
            .iter()
            .all(|p| map::distance(*p, *more[0].1.last().unwrap()) >= map::UNIT_SPACING));
    }
    #[test]
    fn full_perimeter_queues_reinforcements_outside_melee_range() {
        let target = map::BuildingKind::Granary.obstacle(0.0, 9.0);
        let perimeter = map::attack_positions(target)
            .into_iter()
            .filter(|s| s.ring == 0)
            .map(|s| s.point)
            .collect::<Vec<_>>();
        let plans = plan_attack(
            &[(90, (-8.0, 0.0), None), (91, (-8.0, 1.0), None)],
            target,
            &[target],
            &perimeter,
            &perimeter,
        )
        .unwrap();
        for (_, path) in plans {
            let goal = *path.last().unwrap();
            assert!(map::distance(goal, target.closest(goal)) > map::ATTACK_RANGE);
            assert!(perimeter
                .iter()
                .all(|p| map::distance(*p, goal) >= map::UNIT_SPACING));
        }
    }

    #[test]
    fn crowds_cannot_tunnel_or_stack_and_can_pass_a_stationary_unit() {
        let mut p = (-2.0, 0.0);
        let goal = (2.0, 0.0);
        let peers = [(0.0, 0.0)];
        for _ in 0..100 {
            let next = move_separated(p, goal, 0.15, &peers, &[]);
            assert!(segment_distance(peers[0], p, next) >= map::UNIT_SPACING - 0.00002);
            assert!(map::distance(p, next) <= 0.15001);
            p = next;
        }
        assert!(map::distance(p, goal) < 0.01, "{p:?}");
        let next = move_separated((-1.0, 0.0), (1.0, 0.0), 5.0, &peers, &[]);
        assert!(segment_distance(peers[0], (-1.0, 0.0), next) >= map::UNIT_SPACING);
    }
    #[test]
    fn twenty_attackers_reach_distinct_positions_without_collisions() {
        let target = map::BuildingKind::Granary.obstacle(0.0, 9.0);
        let starts = (0..20)
            .map(|i| {
                (
                    i,
                    (-1.4 + (i % 5) as f32 * 0.7, -1.05 + (i / 5) as f32 * 0.7),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let mut paths = plan_attack(&starts, target, &[target], &[], &[])
            .unwrap()
            .into_iter()
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        let mut positions = starts.iter().map(|(_, p, _)| *p).collect::<Vec<_>>();
        for _ in 0..1200 {
            for i in 0..positions.len() {
                while paths[i].len() > 1
                    && can_skip_corner(positions[i], paths[i][0], paths[i][1], &[target])
                {
                    paths[i].remove(0);
                }
                let Some(goal) = paths[i].first().copied() else {
                    continue;
                };
                let peers = positions
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, p)| *p)
                    .collect::<Vec<_>>();
                let p = move_separated(positions[i], goal, 0.15, &peers, &[target]);
                for peer in &peers {
                    assert!(map::distance(p, *peer) >= map::UNIT_SPACING - 0.00002);
                }
                positions[i] = p;
                if map::distance(p, goal) < 0.00001 {
                    paths[i].remove(0);
                }
            }
            if paths.iter().all(Vec::is_empty) {
                return;
            }
        }
        panic!(
            "Attackers failed to surround building: {:?}",
            positions
                .iter()
                .zip(&paths)
                .filter(|(_, p)| !p.is_empty())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unit_can_leave_corner_of_a_stationary_army_without_crossing_it() {
        let peers = (1..20)
            .map(|i| (-1.4 + (i % 5) as f32 * 0.7, -1.05 + (i / 5) as f32 * 0.7))
            .collect::<Vec<_>>();
        let mut p = (-1.4, -1.05);
        let goal = (3.0, 3.0);
        for _ in 0..250 {
            p = move_separated(p, goal, 0.15, &peers, &[]);
            assert!(peers
                .iter()
                .all(|q| map::distance(p, *q) >= map::UNIT_SPACING - 0.00002));
            if map::distance(p, goal) < 0.00001 {
                return;
            }
        }
        panic!("Unit stuck inside stationary army at {p:?}");
    }

    #[test]
    fn crowded_corner_avoids_buildings_and_waiting_units() {
        let obstacle = map::BuildingKind::Granary.obstacle(0.0, 0.0);
        let start = (-4.0, -4.0);
        let peers = [(-3.4, -3.4)];
        let p = move_separated(start, (-3.01, -2.81), 0.15, &peers, &[obstacle]);
        assert!(map::segment_clear(start, p, &[obstacle]));
        assert!(map::distance(p, peers[0]) >= map::UNIT_SPACING);
    }
}
