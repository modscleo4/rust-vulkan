use imgui::{Condition, Ui};
use winit::dpi::PhysicalSize;

use crate::model::Instance;

pub(crate) fn draw_windows(
    ui: &Ui,
    show_debug_menu: bool,
    frame_duration: std::time::Duration,
    elapsed_seconds: f32,
    instances: &[Instance],
    selected_instance: &mut Option<usize>,
    cursor_position: Option<[f64; 2]>,
    window_size: PhysicalSize<u32>,
    scale_factor: f64,
) {
    if !show_debug_menu {
        return;
    }

    let vertices = instances
        .iter()
        .map(|instance| instance.model.vertices.len())
        .sum::<usize>();
    let scene_buffer_bytes = vertices * std::mem::size_of::<crate::model::Vertex>();

    ui.window("Debug Menu (F3)")
        .always_auto_resize(true)
        .position([10.0, 10.0], Condition::FirstUseEver)
        .title_bar(false)
        .resizable(false)
        .build(|| {
            ui.text(format!("Frame time: {:.2} ms", frame_duration.as_secs_f32() * 1000.0));
            ui.text(format!("Frame rate: {:.1} FPS", 1.0 / frame_duration.as_secs_f32()));
            ui.text(format!("Uptime: {elapsed_seconds:.1} seconds"));
            ui.separator();
            ui.text("Scene usage");
            ui.text(format!("Instances: {}", instances.len()));
            ui.text(format!("Vertices: {vertices}"));
            ui.text(format!("Triangles: {}", vertices / 3));
            ui.text(format!(
                "Vertex buffer: {}",
                format_bytes(scene_buffer_bytes)
            ));
        });

    let mut object_window_open = selected_instance.is_some();
    let Some(instance) = selected_instance.and_then(|index| instances.get(index)) else {
        return;
    };

    let model = &instance.model;
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
    let title = format!("Object: {} ({})", model.name, instance.name);
    let mut window = ui
        .window(&title)
        .opened(&mut object_window_open)
        .always_auto_resize(true);
    let object_window_position = cursor_position.map(|position| {
        let logical_position = [
            (position[0] / scale_factor) as f32,
            (position[1] / scale_factor) as f32,
        ];
        object_window_anchor(
            logical_position,
            [
                window_size.width as f32 / scale_factor as f32,
                window_size.height as f32 / scale_factor as f32,
            ],
        )
    });
    if let Some((position, pivot)) = object_window_position {
        window = window
            .position(position, Condition::Appearing)
            .position_pivot(pivot);
    }
    window.build(|| {
        ui.text(format!(
            "Position: ({:.2}, {:.2}, {:.2})",
            instance.position[0], instance.position[1], instance.position[2]
        ));
        ui.text(format!(
            "Rotation: ({:.1}, {:.1}, {:.1}) deg",
            instance.rotation[0].to_degrees(),
            instance.rotation[1].to_degrees(),
            instance.rotation[2].to_degrees(),
        ));
        ui.text(format!(
            "Scale: ({:.2}, {:.2}, {:.2})",
            instance.scale[0], instance.scale[1], instance.scale[2]
        ));
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

    if !object_window_open {
        *selected_instance = None;
    }
}

fn format_bytes(bytes: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

pub(crate) fn object_window_anchor(
    cursor: [f32; 2],
    display_size: [f32; 2],
) -> ([f32; 2], [f32; 2]) {
    let pivot = [
        if cursor[0] < display_size[0] * 0.5 {
            0.0
        } else {
            1.0
        },
        if cursor[1] < display_size[1] * 0.5 {
            0.0
        } else {
            1.0
        },
    ];
    let offset = [
        if pivot[0] == 0.0 { 12.0 } else { -12.0 },
        if pivot[1] == 0.0 { 12.0 } else { -12.0 },
    ];

    ([cursor[0] + offset[0], cursor[1] + offset[1]], pivot)
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, object_window_anchor};

    #[test]
    fn formats_bytes_using_the_largest_suitable_unit() {
        assert_eq!(format_bytes(384), "384 B");
        assert_eq!(format_bytes(1536), "1.50 KiB");
        assert_eq!(format_bytes(2 * 1024 * 1024), "2.00 MiB");
        assert_eq!(format_bytes(1024_usize.pow(3)), "1.00 GiB");
    }

    #[test]
    fn object_window_anchors_away_from_nearest_edges() {
        assert_eq!(
            object_window_anchor([790.0, 590.0], [800.0, 600.0]),
            ([778.0, 578.0], [1.0, 1.0])
        );
        assert_eq!(
            object_window_anchor([10.0, 10.0], [800.0, 600.0]),
            ([22.0, 22.0], [0.0, 0.0])
        );
    }
}
