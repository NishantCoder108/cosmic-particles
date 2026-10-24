
struct Animation {
    values: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> animation: Animation;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) local_position: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Six vertices form a small square for each particle.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );

    let id = f32(instance_index);

    // Deterministic pseudo-random values for each particle.
    let rx = fract(sin(id * 127.1 + 1.0) * 43758.5453);
    let ry = fract(sin(id * 311.7 + 2.0) * 24634.6345);
    let rc = fract(sin(id * 74.7 + 3.0) * 56445.234);

    let base = vec2<f32>(
        rx * 1.8 - 0.9,
        ry * 1.8 - 0.9
    );

    let time = animation.values.x;
    let aspect = max(animation.values.y, 0.1);

    // Give each particle a slightly different flowing motion.
    let phase = id * 0.13;
    let center = base + vec2<f32>(
        sin(time * 0.65 + phase) * 0.09,
        cos(time * 0.48 + phase * 1.7) * 0.09
    );

    let size = 0.004 + rc * 0.006;
    let corner = corners[vertex_index];

    // Compensate for the window's aspect ratio.
    let position = center + vec2<f32>(
        corner.x * size / aspect,
        corner.y * size
    );

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.local_position = corner;

    // A cool cyan-to-purple color palette.
    output.color = mix(
        vec3<f32>(0.05, 0.85, 1.0),
        vec3<f32>(0.75, 0.20, 1.0),
        rc
    );

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance_from_center = length(input.local_position);

    // Discard the square's corners to make circular particles.
    if distance_from_center > 1.0 {
        discard;
    }

    // Fade the edges for a soft glowing appearance.
    let glow = 1.0 - smoothstep(0.25, 1.0, distance_from_center);
    let alpha = glow * 0.9;

    return vec4<f32>(input.color, alpha);
}
