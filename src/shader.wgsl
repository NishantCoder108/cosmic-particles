
struct Animation {
    values: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> animation: Animation;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>( 0.0,  0.6),
        vec2<f32>(-0.6, -0.6),
        vec2<f32>( 0.6, -0.6)
    );

    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0)
    );

    // Read the angle supplied by Rust.
    let angle = animation.values.x;

    let c = cos(angle);
    let s = sin(angle);

    // Rotate the selected vertex around the origin.
    let p = positions[vertex_index];

    let rotated = vec2<f32>(
        c * p.x - s * p.y,
        s * p.x + c * p.y
    );

    var output: VertexOutput;
    output.position = vec4<f32>(rotated, 0.0, 1.0);
    output.color = colors[vertex_index];

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
