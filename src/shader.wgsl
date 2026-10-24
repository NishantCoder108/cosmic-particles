
struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

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
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );

    let particle = particles[instance_index];
    let corner = corners[vertex_index];

    let size = 0.006;
    let position = particle.position + corner * size;

    let id = f32(instance_index);
    let hue = fract(sin(id * 12.9898) * 43758.5453);    

    let cyan = vec3<f32>(0.05, 0.8, 1.0);
    let purple = vec3<f32>(0.45, 0.12, 1.0);
    let pink = vec3<f32>(1.0, 0.08, 0.55);
    let blue = vec3<f32>(0.12, 0.3, 1.0);  

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.local_position = corner;    

    output.color = mix(
        mix(cyan, blue, hue),
        mix(purple, pink, hue),
        hue
    );    

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance_from_center = length(input.local_position);

    if distance_from_center > 1.0 {
        discard;
    }

    let glow = 1.0 - smoothstep(
        0.25,
        1.0,
        distance_from_center
    );

    return vec4<f32>(input.color, glow);
}
