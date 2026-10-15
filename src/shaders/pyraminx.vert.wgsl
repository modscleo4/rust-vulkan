struct Rotation {
    angle: f32,
}

var<push_constant> rotation: Rotation;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn main(input: VertexInput) -> VertexOutput {
    let sine = sin(rotation.angle);
    let cosine = cos(rotation.angle);
    let rotated = vec3<f32>(
        cosine * input.position.x + sine * input.position.z,
        input.position.y,
        -sine * input.position.x + cosine * input.position.z,
    );
    let camera_z = rotated.z + 3.0;
    var output: VertexOutput;
    output.position = vec4<f32>(
        rotated.x * 1.05,
        rotated.y * 1.4,
        camera_z * 1.333333 - 1.333333,
        camera_z,
    );
    output.color = input.color;
    return output;
}
