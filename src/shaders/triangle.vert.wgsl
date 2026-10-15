struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    switch vertex_index {
        case 0u: {
            output.position = vec4<f32>(0.0, -0.6, 0.0, 1.0);
            output.color = vec3<f32>(1.0, 0.2, 0.2);
        }
        case 1u: {
            output.position = vec4<f32>(0.6, 0.6, 0.0, 1.0);
            output.color = vec3<f32>(0.2, 1.0, 0.2);
        }
        default: {
            output.position = vec4<f32>(-0.6, 0.6, 0.0, 1.0);
            output.color = vec3<f32>(0.2, 0.4, 1.0);
        }
    }
    return output;
}
