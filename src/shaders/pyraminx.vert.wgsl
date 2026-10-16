struct Rotation {
    angles: vec4<f32>,
    instance: vec4<f32>,
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
    let sin_x = sin(rotation.angles.x);
    let cos_x = cos(rotation.angles.x);
    let rotated_x = vec3<f32>(
        input.position.x,
        cos_x * input.position.y - sin_x * input.position.z,
        sin_x * input.position.y + cos_x * input.position.z,
    );
    let sin_y = sin(rotation.angles.y);
    let cos_y = cos(rotation.angles.y);
    let rotated_y = vec3<f32>(
        cos_y * rotated_x.x + sin_y * rotated_x.z,
        rotated_x.y,
        -sin_y * rotated_x.x + cos_y * rotated_x.z,
    );
    let sin_z = sin(rotation.angles.z);
    let cos_z = cos(rotation.angles.z);
    let rotated = vec3<f32>(
        cos_z * rotated_y.x - sin_z * rotated_y.y,
        sin_z * rotated_y.x + cos_z * rotated_y.y,
        rotated_y.z,
    );
    let camera_z = rotated.z + 3.0;
    let scale = rotation.instance.z;
    var output: VertexOutput;
    output.position = vec4<f32>(
        rotated.x * 1.05 * scale + rotation.instance.x * camera_z,
        rotated.y * 1.4 * scale + rotation.instance.y * camera_z,
        camera_z * 1.333333 - 1.333333,
        camera_z,
    );
    output.color = input.color;
    return output;
}
