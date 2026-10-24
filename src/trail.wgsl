
@group(0) @binding(0)
var previous_frame: texture_2d<f32>;

@group(0) @binding(1)
var previous_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );

    let position = positions[index];

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = vec2<f32>(
        (position.x + 1.0) * 0.5,
        (1.0 - position.y) * 0.5
    );

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let old_color = textureSample(
        previous_frame,
        previous_sampler,
        input.uv
    );


    // Fade previous pixels by 3% each frame.
    //return vec4<f32>(old_color.rgb * 0.97, 1.0);
    return vec4<f32>(old_color.rgb * 0.95, 1.0);
}
