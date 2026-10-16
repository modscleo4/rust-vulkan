@fragment
fn main(
    @location(0) face_color: vec3<f32>,
    @location(1) outline: f32,
    @location(2) barycentric: vec3<f32>,
) -> @location(0) vec4<f32> {
    let edge_distance = min(
        barycentric.x / fwidth(barycentric.x),
        min(
            barycentric.y / fwidth(barycentric.y),
            barycentric.z / fwidth(barycentric.z),
        ),
    );
    if outline > 0.5 && edge_distance < 2.0 {
        return vec4<f32>(vec3<f32>(1.0) - face_color, 1.0);
    }
    return vec4<f32>(face_color, 1.0);
}
