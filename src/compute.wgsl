
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

    let dt = simulation.delta_time;
    let position = particle.position;

    // Two vortex centers.
    let center_a = vec2<f32>(-0.35, 0.0);
    let center_b = vec2<f32>(0.35, 0.0);

    let offset_a = position - center_a;
    let offset_b = position - center_b;

    // Avoid division by zero near a vortex center.
    let distance_a = max(dot(offset_a, offset_a), 0.025);
    let distance_b = max(dot(offset_b, offset_b), 0.025);

    // Perpendicular force creates circular motion.
    let force_a = vec2<f32>(-offset_a.y, offset_a.x)
        * (0.10 / distance_a);

    let force_b = vec2<f32>(offset_b.y, -offset_b.x)
        * (0.10 / distance_b);

    let force = force_a + force_b;

    // Semi-implicit Euler integration.
    particle.velocity += force * dt;

    // Damping prevents speed from growing without bound.
    particle.velocity *= exp(-0.20 * dt);

    // Cap velocity to keep the simulation stable.
    let speed = length(particle.velocity);
    if speed > 0.8 {
        particle.velocity = particle.velocity * (0.8 / speed);
    }

    particle.position += particle.velocity * dt;

    // Reflect at the simulation boundaries.
    if particle.position.x < -1.0 {
        particle.position.x = -1.0;
        particle.velocity.x = abs(particle.velocity.x);
    } else if particle.position.x > 1.0 {
        particle.position.x = 1.0;
        particle.velocity.x = -abs(particle.velocity.x);
    }

    if particle.position.y < -1.0 {
        particle.position.y = -1.0;
        particle.velocity.y = abs(particle.velocity.y);
    } else if particle.position.y > 1.0 {
        particle.position.y = 1.0;
        particle.velocity.y = -abs(particle.velocity.y);
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