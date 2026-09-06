use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    find_link_route_at, wire_node_editor, LinkData, LinkPath, NodeEditorSetup,
    PolylineLinkRoute,
};
use std::rc::Rc;

slint::include_modules!();

/// Generate an orthogonal (Manhattan) path: Horizontal -> Vertical -> Horizontal
/// Uses pure world coordinates - zoom is handled by the container's transform-scale
///
/// A custom shape owes the editor its bounding box as well as its commands:
/// the `Link` element is sized to the box, so anything drawn outside it is
/// left stale by partial rendering. The staircase turns at the midpoint in x
/// and never leaves the rectangle the two endpoints span, so that rectangle is
/// the box.
fn manhattan_points(
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
) -> [(f32, f32); 4] {
    let mid_x = (start_x + end_x) / 2.0;
    [
        (start_x, start_y),
        (mid_x, start_y),
        (mid_x, end_y),
        (end_x, end_y),
    ]
}

fn generate_manhattan_path(start_x: f32, start_y: f32, end_x: f32, end_y: f32) -> LinkPath {
    let points = manhattan_points(start_x, start_y, end_x, end_y);
    let x = start_x.min(end_x);
    let y = start_y.min(end_y);

    // M sx sy -> Move to start
    // L mid_x sy -> Line to first corner
    // L mid_x ey -> Line to second corner
    // L ex ey -> Line to end
    LinkPath {
        commands: format!(
            "M {} {} L {} {} L {} {} L {} {}",
            points[0].0 - x,
            points[0].1 - y,
            points[1].0 - x,
            points[1].1 - y,
            points[2].0 - x,
            points[2].1 - y,
            points[3].0 - x,
            points[3].1 - y
        )
        .into(),
        x,
        y,
        width: (end_x - start_x).abs(),
        height: (end_y - start_y).abs(),
    }
}

fn main() {
    let window = MainWindow::new().unwrap();
    let w = window.as_weak();

    // Set up nodes
    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Node A"),
            x: 100.0,
            y: 100.0,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Node B"),
            x: 450.0,
            y: 250.0,
        },
        NodeData {
            id: 3,
            title: SharedString::from("Node C"),
            x: 100.0,
            y: 400.0,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up links
    let links = Rc::new(VecModel::from(vec![
        LinkData {
            id: 1,
            start_pin_id: 3, // Node 1 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 100, 180, 255),
            line_width: 2.0,
            status: -1,
            selected: false,
        },
        LinkData {
            id: 2,
            start_pin_id: 7, // Node 3 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 255, 180, 100),
            line_width: 3.0, // Thicker link
            status: -1,
            selected: false,
        },
    ]));
    window.set_links(ModelRc::from(links.clone()));

    // Create setup with model update logic
    let setup = NodeEditorSetup::new({
        let nodes = nodes.clone();
        move |node_id, delta_x, delta_y| {
            for i in 0..nodes.row_count() {
                if let Some(mut node) = nodes.row_data(i) {
                    if node.id == node_id {
                        node.x += delta_x;
                        node.y += delta_y;
                        nodes.set_row_data(i, node);
                        break;
                    }
                }
            }
        }
    });

    // Wire all standard callbacks with one macro call
    wire_node_editor!(window, setup);

    // Custom link path computation via global callback
    window
        .global::<NodeEditorComputations>()
        .on_compute_link_path({
            let ctrl = setup.controller().clone();
            let w = w.clone();
            move |start_pin, end_pin, _version| {
                let w = match w.upgrade() {
                    Some(w) => w,
                    None => return LinkPath::default(),
                };
                let style = w.get_link_style();
                let bezier_offset = w.get_bezier_min_offset();

                let cache = ctrl.cache();
                let cache = cache.borrow();
                let Some((sx, sy, ex, ey)) =
                    cache.resolve_link_endpoints_world(start_pin, end_pin)
                else {
                    return LinkPath::default();
                };

                if style == "orthogonal" {
                    generate_manhattan_path(sx, sy, ex, ey)
                } else {
                    cache
                        .link_curve_world(start_pin, end_pin, bezier_offset)
                        .map(|curve| {
                            curve.to_link_path(|commands, x, y, width, height| LinkPath {
                                commands: commands.into(),
                                x,
                                y,
                                width,
                                height,
                            })
                        })
                        .unwrap_or_default()
                }
            }
        });

    window.on_compute_link_at({
        let ctrl = setup.controller().clone();
        let links = links.clone();
        let w = w.clone();
        move |world_x, world_y| {
            let Some(w) = w.upgrade() else {
                return -1;
            };
            let world_hover_distance =
                ctrl.screen_distance_to_world(w.get_link_hover_distance());
            let hit_samples = w.get_link_hit_samples() as usize;
            let rows = (0..links.row_count()).filter_map(|i| links.row_data(i));
            let cache = ctrl.cache();
            let cache = cache.borrow();

            if w.get_link_style() == "orthogonal" {
                let routes = rows.filter_map(|link| {
                    let (sx, sy, ex, ey) = cache.resolve_link_endpoints_world(
                        link.start_pin_id,
                        link.end_pin_id,
                    )?;
                    Some(PolylineLinkRoute {
                        id: link.id,
                        points: manhattan_points(sx, sy, ex, ey),
                    })
                });
                find_link_route_at(
                    (world_x, world_y),
                    routes,
                    world_hover_distance,
                    hit_samples,
                )
            } else {
                let rows = rows.map(|link| (link.id, link.start_pin_id, link.end_pin_id));
                cache.find_bezier_link_at_world(
                    world_x,
                    world_y,
                    rows,
                    world_hover_distance,
                    w.get_bezier_min_offset(),
                    hit_samples,
                )
            }
        }
    });

    window.run().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthogonal_picker_uses_the_rendered_segments() {
        let route = PolylineLinkRoute {
            id: 7,
            points: manhattan_points(100.0, 80.0, -20.0, 240.0),
        };

        for zoom in [0.1, 0.25, 1.0, 3.0] {
            let world_hover_distance = 8.0 / zoom;
            assert_eq!(
                find_link_route_at((40.0, 170.0), [route], world_hover_distance, 0),
                7,
            );
            assert_eq!(
                find_link_route_at(
                    (40.0 + 9.0 / zoom, 170.0),
                    [route],
                    world_hover_distance,
                    0,
                ),
                -1,
                "nine-screen-pixel offset hit at zoom {zoom}",
            );
        }
    }
}
