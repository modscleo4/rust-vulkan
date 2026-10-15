@fragment
fn main(@location(0) vertex_color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(vertex_color, 1.0);
}
