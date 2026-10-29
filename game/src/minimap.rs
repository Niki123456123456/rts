use crate::*;

pub(crate) struct MinimapSize {
    side: f32,
    drag_start: f32,
}

impl Default for MinimapSize {
    fn default() -> Self {
        Self {
            side: 240.0,
            drag_start: 240.0,
        }
    }
}

fn map_position(rect: egui::Rect, x: f32, z: f32) -> egui::Pos2 {
    let half = stronghold_map::MAP_SIZE * 0.5;
    egui::pos2(
        rect.left() + (x + half) / stronghold_map::MAP_SIZE * rect.width(),
        rect.top() + (z + half) / stronghold_map::MAP_SIZE * rect.height(),
    )
}

fn world_position(rect: egui::Rect, position: egui::Pos2) -> Vec2 {
    let half = stronghold_map::MAP_SIZE * 0.5;
    Vec2::new(
        ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * stronghold_map::MAP_SIZE
            - half,
        ((position.y - rect.top()) / rect.height()).clamp(0.0, 1.0) * stronghold_map::MAP_SIZE
            - half,
    )
}

pub(crate) fn minimap_ui(
    mut contexts: EguiContexts,
    multiplayer: Res<Multiplayer>,
    mut camera: Single<(&mut RtsCamera, &Camera, &GlobalTransform)>,
    mut controls: Query<&mut RtsCameraControls>,
    mut texture: Local<Option<egui::TextureHandle>>,
    mut dimensions: Local<MinimapSize>,
    mut panel: ResMut<SelectionPanel>,
) -> Result {
    if multiplayer.snapshot.game_id.is_none() || !multiplayer.snapshot.connected {
        panel.1 = None;
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    // Cache a terrain image at one sample per world metre. Use the same surface
    // function as construction previews so hills and raised ground stay aligned.
    let terrain = texture.get_or_insert_with(|| {
        let size = stronghold_map::MAP_SIZE.ceil() as usize;
        let mut pixels = Vec::with_capacity(size * size);
        let half = stronghold_map::MAP_SIZE * 0.5;
        for row in 0..size {
            for column in 0..size {
                let x = (column as f32 + 0.5) / size as f32 * stronghold_map::MAP_SIZE - half;
                let z = (row as f32 + 0.5) / size as f32 * stronghold_map::MAP_SIZE - half;
                let [r, g, b] = stronghold_map::terrain::color(x, z);
                let shade = (stronghold_map::terrain::height(x, z)
                    - stronghold_map::terrain::height(x - 2.0, z - 2.0))
                    * 8.0;
                let channel = |v: u8| (v as f32 + shade.clamp(-28.0, 28.0)).clamp(0.0, 255.0) as u8;
                let color = egui::Color32::from_rgb(channel(r), channel(g), channel(b));
                pixels.push(color);
            }
        }
        ctx.load_texture(
            "minimap_terrain",
            egui::ColorImage::new([size, size], pixels),
            egui::TextureOptions::LINEAR,
        )
    });
    let (rig, view, transform) = &mut *camera;
    // Use the complete viewport: the minimap deliberately covers the action bar.
    let viewport = ctx.viewport_rect();
    let maximum = (viewport.width().min(viewport.height()) * 0.8).max(64.0);
    let minimum = 120.0_f32.min(maximum);
    dimensions.side = dimensions.side.clamp(minimum, maximum);
    let size = dimensions.side;
    let response = egui::Area::new(egui::Id::new("minimap_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(viewport.right_bottom() - egui::vec2(size, size))
        .constrain(false)
        .movable(false)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let grip_rect = egui::Rect::from_min_size(rect.min, egui::vec2(22.0, 22.0));
            // Keep navigation and resizing hit areas disjoint.
            let response = ui
                .interact(
                    egui::Rect::from_min_max(rect.min + egui::vec2(22.0, 0.0), rect.max),
                    ui.id().with("navigate_right"),
                    egui::Sense::click_and_drag(),
                )
                .union(ui.interact(
                    egui::Rect::from_min_max(
                        rect.min + egui::vec2(0.0, 22.0),
                        egui::pos2(rect.left() + 22.0, rect.bottom()),
                    ),
                    ui.id().with("navigate_left"),
                    egui::Sense::click_and_drag(),
                ));
            let grip = ui
                .interact(grip_rect, ui.id().with("resize"), egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
                .on_hover_text("Ziehen, um die Minimap zu vergrößern oder zu verkleinern");
            if grip.drag_started() {
                dimensions.drag_start = size;
            }
            if grip.dragged() {
                let delta = grip.total_drag_delta().unwrap_or_default();
                let change = if delta.x.abs() > delta.y.abs() {
                    delta.x
                } else {
                    delta.y
                };
                dimensions.side = (dimensions.drag_start - change).clamp(minimum, maximum);
            }
            painter.image(
                terrain.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            for building in &multiplayer.snapshot.buildings {
                if building.health <= 0.0 {
                    continue;
                }
                let color = player_color(building.color).to_srgba();
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        map_position(rect, building.x, building.z),
                        egui::vec2(4.0, 4.0),
                    ),
                    0.0,
                    egui::Color32::from_rgb(
                        (color.red * 255.0) as u8,
                        (color.green * 255.0) as u8,
                        (color.blue * 255.0) as u8,
                    ),
                );
            }
            for unit in &multiplayer.snapshot.units {
                if unit.health <= 0.0 {
                    continue;
                }
                let color = player_color(unit.color).to_srgba();
                painter.circle_filled(
                    map_position(rect, unit.x, unit.z),
                    1.8,
                    egui::Color32::from_rgb(
                        (color.red * 255.0) as u8,
                        (color.green * 255.0) as u8,
                        (color.blue * 255.0) as u8,
                    ),
                );
            }
            // Project the camera's screen corners onto the ground plane to show
            // its actual visible footprint, including camera rotation and zoom.
            if let Some(viewport) = view.logical_viewport_rect() {
                let corners = [
                    viewport.min,
                    Vec2::new(viewport.max.x, viewport.min.y),
                    viewport.max,
                    Vec2::new(viewport.min.x, viewport.max.y),
                ];
                let points: Vec<_> = corners
                    .into_iter()
                    .filter_map(|corner| {
                        let ray = view.viewport_to_world(transform, corner).ok()?;
                        if ray.direction.y >= -0.0001 {
                            return None;
                        }
                        let distance = (rig.focus.translation.y - ray.origin.y) / ray.direction.y;
                        if distance < 0.0 {
                            return None;
                        }
                        let point = ray.get_point(distance);
                        Some(map_position(rect, point.x, point.z))
                    })
                    .collect();
                if points.len() == 4 {
                    painter.add(egui::Shape::closed_line(
                        points,
                        egui::Stroke::new(1.2, egui::Color32::WHITE),
                    ));
                }
            }
            painter.circle_stroke(
                map_position(
                    rect,
                    rig.target_focus.translation.x,
                    rig.target_focus.translation.z,
                ),
                4.0,
                egui::Stroke::new(1.3, egui::Color32::YELLOW),
            );
            painter.text(
                rect.left_top() + egui::vec2(28.0, 5.0),
                egui::Align2::LEFT_TOP,
                "N",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(35, 38, 34)),
                egui::StrokeKind::Inside,
            );
            for offset in [5.0, 10.0, 15.0] {
                painter.line_segment(
                    [
                        rect.min + egui::vec2(3.0, offset),
                        rect.min + egui::vec2(offset, 3.0),
                    ],
                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                );
            }
            let over_grip = ctx
                .pointer_interact_pos()
                .is_some_and(|p| grip_rect.contains(p));
            if !over_grip
                && !grip.dragged()
                && !grip.drag_stopped()
                && (response.clicked_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary))
            {
                if let Some(position) = response.interact_pointer_pos() {
                    let point = world_position(rect, position).clamp(
                        Vec2::splat(-stronghold_map::MAP_LIMIT),
                        Vec2::splat(stronghold_map::MAP_LIMIT),
                    );
                    rig.target_focus.translation.x = point.x;
                    rig.target_focus.translation.z = point.y;
                    rig.target_focus.translation.y =
                        stronghold_map::terrain::height(point.x, point.y);
                    rig.snap = true;
                }
            }
            if let Some(position) = response.hover_pos() {
                let world = world_position(rect, position);
                response.on_hover_text(format!(
                    "{} · Klicken oder ziehen, um die Kamera zu bewegen",
                    stronghold_map::terrain::biome(world.x, world.y).label()
                ));
            }
        });
    panel.1 = Some(response.response.rect);
    if ctx
        .pointer_interact_pos()
        .is_some_and(|position| response.response.rect.contains(position))
    {
        for mut control in &mut controls {
            control.enabled = false;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_coordinates_round_trip_and_clamp() {
        let rect = egui::Rect::from_min_size(egui::pos2(13.0, 29.0), egui::vec2(240.0, 240.0));
        for (x, z) in [(-400.0, -400.0), (0.0, 0.0), (400.0, 400.0), (137.0, -89.0)] {
            let point = world_position(rect, map_position(rect, x, z));
            assert!((point.x - x).abs() < 0.001);
            assert!((point.y - z).abs() < 0.001);
        }
        assert_eq!(
            world_position(rect, rect.min - egui::vec2(100.0, 100.0)),
            Vec2::splat(-400.0)
        );
        assert_eq!(
            world_position(rect, rect.max + egui::vec2(100.0, 100.0)),
            Vec2::splat(400.0)
        );
    }
}
