use slint::{Color, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_node_editor::NodeEditorController;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

slint::include_modules!();

// Animation duration in seconds
const ANIMATION_DURATION: f32 = 0.5;

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Track animation start time
    let start_time = Instant::now();

    // Set up nodes with different positions for a nice layout
    // Positioned to the right to avoid overlay
    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Source"),
            x: 350.0,
            y: 80.0,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Processor"),
            x: 580.0,
            y: 50.0,
        },
        NodeData {
            id: 3,
            title: SharedString::from("Filter"),
            x: 580.0,
            y: 180.0,
        },
        NodeData {
            id: 4,
            title: SharedString::from("Output"),
            x: 810.0,
            y: 115.0,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up animated links storage
    let animated_links: Rc<VecModel<AnimatedLinkData>> = Rc::new(VecModel::default());
    window.set_animated_links(ModelRc::from(animated_links.clone()));

    // Link ID counter
    let next_link_id = Rc::new(RefCell::new(1));

    // Color palette for new links (cycles through these)
    let link_colors = [
        Color::from_argb_u8(255, 255, 107, 107), // Coral red
        Color::from_argb_u8(255, 78, 205, 196),  // Teal
        Color::from_argb_u8(255, 255, 230, 109), // Yellow
        Color::from_argb_u8(255, 168, 85, 247),  // Purple
        Color::from_argb_u8(255, 74, 158, 255),  // Blue
        Color::from_argb_u8(255, 52, 211, 153),  // Green
    ];

    // Core callbacks
    window.on_compute_link_path(ctrl.compute_link_path_callback());
    window.on_node_drag_started(ctrl.node_drag_started_callback());

    // Pin hit testing
    window.on_compute_pin_at({
        let ctrl = ctrl.clone();
        move |x, y| ctrl.cache().borrow().find_pin_at(x as f32, y as f32, 10.0)
    });

    // Link preview path generation
    window.on_compute_link_preview_path({
        let ctrl = ctrl.clone();
        move |start_x, start_y, end_x, end_y| {
            slint_node_editor::generate_bezier_path(
                start_x as f32,
                start_y as f32,
                end_x as f32,
                end_y as f32,
                ctrl.zoom(),
                50.0,
            )
            .into()
        }
    });

    // Animated link path generation (partial bezier based on progress)
    window.on_compute_animated_link_path({
        let ctrl = ctrl.clone();
        move |start_pin, end_pin, progress, _version| {
            let cache = ctrl.cache();
            let cache = cache.borrow();

            // Get pin positions from cache
            let start_pos = match cache.pin_positions.get(&start_pin) {
                Some(p) => p,
                None => return SharedString::default(),
            };
            let end_pos = match cache.pin_positions.get(&end_pin) {
                Some(p) => p,
                None => return SharedString::default(),
            };

            // Get node rects to calculate absolute positions
            let start_rect = match cache.node_rects.get(&start_pos.node_id) {
                Some(r) => r,
                None => return SharedString::default(),
            };
            let end_rect = match cache.node_rects.get(&end_pos.node_id) {
                Some(r) => r,
                None => return SharedString::default(),
            };

            // Calculate absolute screen positions
            let zoom = ctrl.zoom();
            let start_x = start_rect.x + start_pos.rel_x;
            let start_y = start_rect.y + start_pos.rel_y;
            let end_x = end_rect.x + end_pos.rel_x;
            let end_y = end_rect.y + end_pos.rel_y;

            slint_node_editor::generate_partial_bezier_path(
                start_x, start_y, end_x, end_y, zoom, 50.0, progress,
            )
            .into()
        }
    });

    // Handle animated link creation
    window.on_add_animated_link({
        let animated_links = animated_links.clone();
        let next_link_id = next_link_id.clone();
        let start_time = start_time;
        let link_colors = link_colors;
        move |from_pin, to_pin| {
            // Get unique ID and increment
            let id = *next_link_id.borrow();
            *next_link_id.borrow_mut() = id + 1;

            // Pick color based on ID
            let color = link_colors[(id as usize - 1) % link_colors.len()];

            // Record birth time for animation effects
            let birth_time = start_time.elapsed().as_secs_f32();

            animated_links.push(AnimatedLinkData {
                id,
                start_pin_id: from_pin,
                end_pin_id: to_pin,
                color,
                line_width: 2.5,
                progress: 0.0, // Start invisible, will animate to 1.0
                birth_time,
            });
        }
    });

    // Geometry tracking
    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
        }
    });

    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        move |pid, nid, ptype, x, y| {
            ctrl.handle_pin_position(pid, nid, ptype, x, y);
        }
    });

    // Grid updates
    window.on_request_grid_update({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move || {
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
            }
        }
    });

    window.on_update_viewport({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move |z, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_zoom(z);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });

    // Node drag handling
    window.on_node_drag_ended({
        let ctrl = ctrl.clone();
        move |delta_x, delta_y| {
            let node_id = ctrl.dragged_node_id();
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

    // Animation timer - updates animation time and link progress
    let animation_timer = Timer::default();
    animation_timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(16), // ~60fps
        {
            let w = w.clone();
            let animated_links = animated_links.clone();
            let start_time = start_time;
            move || {
                if let Some(w) = w.upgrade() {
                    let elapsed = start_time.elapsed().as_secs_f32();
                    w.set_animation_time(elapsed);

                    // Update progress for animating links
                    for i in 0..animated_links.row_count() {
                        if let Some(mut link) = animated_links.row_data(i) {
                            if link.progress < 1.0 {
                                // Calculate progress based on time since birth
                                let age = elapsed - link.birth_time;
                                let new_progress = (age / ANIMATION_DURATION).min(1.0);

                                // Apply easing (ease-out cubic)
                                let eased = 1.0 - (1.0 - new_progress).powi(3);

                                link.progress = eased;
                                animated_links.set_row_data(i, link);
                            }
                        }
                    }
                }
            }
        },
    );

    window.invoke_request_grid_update();
    window.run().unwrap();
}
