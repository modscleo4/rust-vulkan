use crate::{config::CameraConfig, model::Instance};

pub(crate) fn pick_object(
    instances: &[Instance],
    cursor: [f64; 2],
    width: u32,
    height: u32,
    camera: &CameraConfig,
) -> Option<usize> {
    let [ndc_x, ndc_y] = screen_to_ndc(cursor, width, height)?;
    instances
        .iter()
        .enumerate()
        .find_map(|(instance_index, instance)| {
            if instance
                .scale
                .iter()
                .any(|scale| scale.abs() < f32::EPSILON)
            {
                return None;
            }
            let ray_direction = [
                (ndc_x - instance.position[0]) / camera.projection[0],
                (ndc_y - instance.position[1]) / camera.projection[1],
                1.0,
            ];
            let ray_origin = rotate_to_object_space(
                [0.0, 0.0, -camera.distance - instance.position[2]],
                instance.rotation,
            );
            let ray_direction = rotate_to_object_space(ray_direction, instance.rotation);
            let ray_origin = divide_by_scale(ray_origin, instance.scale);
            let ray_direction = divide_by_scale(ray_direction, instance.scale);
            instance
                .model
                .vertices
                .chunks_exact(3)
                .any(|triangle| {
                    ray_intersects_triangle(
                        ray_origin,
                        ray_direction,
                        triangle[0].position,
                        triangle[1].position,
                        triangle[2].position,
                    )
                })
                .then_some(instance_index)
        })
}

fn screen_to_ndc(cursor: [f64; 2], width: u32, height: u32) -> Option<[f32; 2]> {
    if width == 0 || height == 0 {
        return None;
    }

    Some([
        (2.0 * cursor[0] / f64::from(width) - 1.0) as f32,
        (1.0 - 2.0 * cursor[1] / f64::from(height)) as f32,
    ])
}

fn ray_intersects_triangle(
    origin: [f32; 3],
    direction: [f32; 3],
    vertex_a: [f32; 3],
    vertex_b: [f32; 3],
    vertex_c: [f32; 3],
) -> bool {
    let subtract = |left: [f32; 3], right: [f32; 3]| {
        [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
    };
    let cross = |left: [f32; 3], right: [f32; 3]| {
        [
            left[1] * right[2] - left[2] * right[1],
            left[2] * right[0] - left[0] * right[2],
            left[0] * right[1] - left[1] * right[0],
        ]
    };
    let dot = |left: [f32; 3], right: [f32; 3]| {
        left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
    };

    let edge_a = subtract(vertex_b, vertex_a);
    let edge_b = subtract(vertex_c, vertex_a);
    let perpendicular = cross(direction, edge_b);
    let determinant = dot(edge_a, perpendicular);
    if determinant.abs() < 1e-6 {
        return false;
    }

    let inverse_determinant = determinant.recip();
    let from_vertex = subtract(origin, vertex_a);
    let u = dot(from_vertex, perpendicular) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }

    let perpendicular = cross(from_vertex, edge_a);
    let v = dot(direction, perpendicular) * inverse_determinant;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }

    dot(edge_b, perpendicular) * inverse_determinant > 0.0
}

fn rotate_to_object_space(point: [f32; 3], angles: [f32; 3]) -> [f32; 3] {
    let [angle_x, angle_y, angle_z] = angles;
    let (sin_z, cos_z) = (-angle_z).sin_cos();
    let rotated_z = [
        cos_z * point[0] - sin_z * point[1],
        sin_z * point[0] + cos_z * point[1],
        point[2],
    ];
    let (sin_y, cos_y) = (-angle_y).sin_cos();
    let rotated_y = [
        cos_y * rotated_z[0] + sin_y * rotated_z[2],
        rotated_z[1],
        -sin_y * rotated_z[0] + cos_y * rotated_z[2],
    ];
    let (sin_x, cos_x) = (-angle_x).sin_cos();
    [
        rotated_y[0],
        cos_x * rotated_y[1] - sin_x * rotated_y[2],
        sin_x * rotated_y[1] + cos_x * rotated_y[2],
    ]
}

fn divide_by_scale(point: [f32; 3], scale: [f32; 3]) -> [f32; 3] {
    [
        point[0] / scale[0],
        point[1] / scale[1],
        point[2] / scale[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_triangle_intersection_detects_hits_and_misses() {
        let triangle = [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]];

        assert!(ray_intersects_triangle(
            [0.0, 0.0, -2.0],
            [0.0, 0.0, 1.0],
            triangle[0],
            triangle[1],
            triangle[2],
        ));
        assert!(!ray_intersects_triangle(
            [2.0, 2.0, -2.0],
            [0.0, 0.0, 1.0],
            triangle[0],
            triangle[1],
            triangle[2],
        ));
    }

    #[test]
    fn inverse_rotation_transforms_the_camera_origin() {
        let rotated =
            rotate_to_object_space([0.0, 0.0, -3.0], [std::f32::consts::FRAC_PI_2, 0.0, 0.0]);

        assert!(rotated[0].abs() < 1e-6);
        assert!((rotated[1] + 3.0).abs() < 1e-6);
        assert!(rotated[2].abs() < 1e-6);
    }

    #[test]
    fn screen_to_ndc_maps_top_to_positive_y() {
        assert_eq!(screen_to_ndc([400.0, 0.0], 800, 600), Some([0.0, 1.0]));
        assert_eq!(screen_to_ndc([400.0, 600.0], 800, 600), Some([0.0, -1.0]));
        assert_eq!(screen_to_ndc([0.0, 0.0], 0, 600), None);
    }
}
