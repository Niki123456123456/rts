pub(crate) fn step_towards(x: f32, z: f32, target_x: f32, target_z: f32, step: f32) -> (f32, f32) {
    let (dx, dz) = (target_x - x, target_z - z);
    let distance = dx.hypot(dz);
    if distance <= step {
        (target_x, target_z)
    } else {
        (x + dx / distance * step, z + dz / distance * step)
    }
}

// Matches the plane, two cuboids and sphere created by the Bevy scene.
pub(crate) fn terrain_height(x: f32, z: f32) -> f32 {
    let mut height: f32 = 0.0;
    if (7.5..=22.5).contains(&x) && (-7.5..=-2.5).contains(&z) {
        height = 1.0;
    }
    if (-20.0..=-10.0).contains(&x) && (-7.5..=7.5).contains(&z) {
        height = 5.0;
    }
    let sphere_height_squared = 12.5_f32.powi(2) - x * x - (z + 23.0).powi(2);
    if sphere_height_squared >= 0.0 {
        height = height.max(sphere_height_squared.sqrt());
    }
    height
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
    fn terrain_matches_scene_geometry() {
        assert_eq!(terrain_height(0.0, 0.0), 0.0);
        assert_eq!(terrain_height(15.0, -5.0), 1.0);
        assert_eq!(terrain_height(-15.0, 0.0), 5.0);
        assert_eq!(terrain_height(0.0, -23.0), 12.5);
    }
}
