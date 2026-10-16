use imgui::{Condition, Ui};

use crate::model::Model;

pub(crate) fn draw_windows(
    ui: &Ui,
    show_debug_menu: bool,
    elapsed_seconds: f32,
    angle: f32,
    model: &Model,
    object_window_open: &mut bool,
    object_window_position: Option<[f32; 2]>,
) {
    if !show_debug_menu {
        return;
    }

    ui.window("Debug Menu (F3)")
        .always_auto_resize(true)
        .position([10.0, 10.0], Condition::FirstUseEver)
        .title_bar(false)
        .resizable(false)
        .build(|| {
            ui.text(format!("Frame rate: {:.1} FPS", ui.io().framerate));
            ui.text(format!("Uptime: {elapsed_seconds:.1} seconds"));
            ui.text(format!("Scene vertices: {}", model.vertices.len()));
        });

    if !*object_window_open {
        return;
    }

    let min = model
        .vertices
        .iter()
        .fold([f32::INFINITY; 3], |mut bounds, vertex| {
            for (axis, value) in vertex.position.iter().enumerate() {
                bounds[axis] = bounds[axis].min(*value);
            }
            bounds
        });
    let max = model
        .vertices
        .iter()
        .fold([f32::NEG_INFINITY; 3], |mut bounds, vertex| {
            for (axis, value) in vertex.position.iter().enumerate() {
                bounds[axis] = bounds[axis].max(*value);
            }
            bounds
        });
    let mut is_open = *object_window_open;
    let title = format!("Object: {}", model.name);
    let mut window = ui
        .window(&title)
        .opened(&mut is_open)
        .always_auto_resize(true);
    if let Some(position) = object_window_position {
        window = window.position(position, Condition::Appearing);
    }
    window.build(|| {
        ui.text("Position: (0.00, 0.00, 0.00)");
        ui.text(format!(
            "Rotation: (0.00, {:.1}, 0.00) deg",
            angle.to_degrees()
        ));
        ui.text("Scale: (1.00, 1.00, 1.00)");
        ui.separator();
        ui.text(format!("Vertices: {}", model.vertices.len()));
        ui.text(format!("Triangles: {}", model.vertices.len() / 3));
        ui.text(format!(
            "Bounds min: ({:.2}, {:.2}, {:.2})",
            min[0], min[1], min[2]
        ));
        ui.text(format!(
            "Bounds max: ({:.2}, {:.2}, {:.2})",
            max[0], max[1], max[2]
        ));
    });
    *object_window_open = is_open;
}
