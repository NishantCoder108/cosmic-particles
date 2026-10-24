
struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

struct Simulation {
    delta_time: f32,
    particle_count: u32,
    _padding: vec2<u32>,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> simulation: Simulation;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;

    if (index >= simulation.particle_count) {
        return;
    }

    var particle = particles[index];

    // Integrate position using velocity and elapsed frame time.
    particle.position += particle.velocity * simulation.delta_time;

    // Bounce off the simulation area's boundaries.
    if (particle.position.x < -1.0 || particle.position.x > 1.0) {
        particle.velocity.x = -particle.velocity.x;
        particle.position.x = clamp(particle.position.x, -1.0, 1.0);
    }

    if (particle.position.y < -1.0 || particle.position.y > 1.0) {
        particle.velocity.y = -particle.velocity.y;
        particle.position.y = clamp(particle.position.y, -1.0, 1.0);
    }

    particles[index] = particle;
}


//--------------
// What this shader does:
//
// - @compute marks this function as a compute shader, not a vertex or fragment shader.
// - @workgroup_size(64) groups invocations into workgroups of 64.
// - global_invocation_id.x identifies which particle a particular invocation updates.
// - The bounds check prevents extra invocations from accessing particles outside the buffer.
//