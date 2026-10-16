struct Rotation {
    angles: vec4<f32>,
    position: vec4<f32>,
    scale_and_outline: vec4<f32>,
    camera: vec4<f32>,
}

var<push_constant> rotation: Rotation;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) outline: f32,
    @location(2) barycentric: vec3<f32>,
}

@vertex
fn main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let scaled = input.position * rotation.scale_and_outline.xyz;
    let sin_x = sin(rotation.angles.x);
    let cos_x = cos(rotation.angles.x);
    let rotated_x = vec3<f32>(
        scaled.x,
        cos_x * scaled.y - sin_x * scaled.z,
        sin_x * scaled.y + cos_x * scaled.z,
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
    let camera_z = rotated.z + rotation.position.z + rotation.position.w;
    var output: VertexOutput;
    output.position = vec4<f32>(
        rotated.x * rotation.camera.x + rotation.position.x * camera_z,
        rotated.y * rotation.camera.y + rotation.position.y * camera_z,
        camera_z * rotation.camera.z - rotation.camera.z,
        camera_z,
    );
    output.color = input.color;
    output.outline = rotation.scale_and_outline.w;
    let vertex_in_triangle = vertex_index % 3u;
    if vertex_in_triangle == 0u {
        output.barycentric = vec3<f32>(1.0, 0.0, 0.0);
    } else if vertex_in_triangle == 1u {
        output.barycentric = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        output.barycentric = vec3<f32>(0.0, 0.0, 1.0);
    }
    return output;
}
