@fragment
fn main(@location(0) face_color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(face_color, 1.0);
}
