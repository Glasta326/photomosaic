struct ProgramState {
    surivial_threshold: u32,
    child_count: u32,
    mutation_strength: f32,
    hue_enabled: u32,
    random_seed: u32,
    target_dimensions_x: u32,
    target_dimensions_y: u32,
}

struct Candidate {
    texture_id: u32,
    pos_x: f32,
    pos_y: f32,
    rotation: f32,
    scale: f32,
    hue: f32,
};

const TAU = 6.28318530717958647692528676655900577;
const PI = 3.14159265358979323846264338327950288;
@group(0) @binding(0) var<uniform> state: ProgramState;
@group(0) @binding(1) var<storage,read> input_candidates: array<Candidate>;
@group(0) @binding(2) var<storage,read_write> output_candidates: array<Candidate>;

@compute @workgroup_size(16,16)
fn process(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let parent_index = global_id.x;
    let child_index = global_id.y;
    let output_index = parent_index * (state.child_count + 1) + child_index;
    let parent_candidate = input_candidates[parent_index];

    // oob is bad!
    if (parent_index >= state.surivial_threshold || child_index >= state.child_count + 1){
        return;
    }

    var rng = state.random_seed + output_index;
    

    // Insert the parent into the end slot
    if child_index == state.child_count {
        output_candidates[output_index] = parent_candidate;
    }
    else {
        output_candidates[output_index] = gen_child_candidate(parent_candidate, &rng);
    }

    // OLD VERSION:
    // This used to be a simple 1d shader that looped over each child
    // Now it is a 2d shader where each x is the parent and each y is all the children of that parent, at once
    // ill clear this comment below soon i just want to keep it for archiving purposes

    // // each thread needs to handle a section of the array that matches childcount + parent, so we need to offset the "thread index" untill we get to our space
    // // for example, if child count is 99, and we're on thread 8, we need to move to position 800, and operate on the output buffer from index 800 to 899 
    // let index = thread_index * (state.child_count + 1);
    // let boundary = index + state.child_count;

    // Initialise the prng with our properly random seed from the program and offset it by our index so each thread doesnt get the exact same sequence
    // var rng = state.random_seed + index;

    // // Populate n - 1 slots with all the children
    // for (var i: u32 = index; i < boundary; i++) {
    //     output_candidates[i] = gen_child_candidate(parent_candidate, &rng);
    // }
    // // Insert the parent into the final slot
    // output_candidates[boundary] = parent_candidate;
}

fn gen_child_candidate(parent: Candidate, rng: ptr<function,u32>) -> Candidate {
    // Generate a random position inside the canvas,
    // then, lerp between our current position x,y to the new position x,y individiually to get the final position
    let random_pos = random_point_2d(vec2<u32>(state.target_dimensions_x, state.target_dimensions_y), rng);
    let new_pos = vec2<f32>(
        lerp(parent.pos_x, random_pos.x, state.mutation_strength),
        lerp(parent.pos_y, random_pos.y, state.mutation_strength)
    );

    // For angle, we want to either rotate left or right, and an amount decided by a range, with the limit on that range being affected by mutation strength
    // so for example, if we are at angle 0.0, we randomly choose to rotate clockwise, and mutation strength is 0.2, so we pick an amount to rotate
    // between 0 and +PI * 0.2
    let random_off = f32(coinflip(rng)) * ((next_f32(rng) * PI) * state.mutation_strength);
    let new_ang = wrap_angle(parent.rotation + random_off);

    // Scale is slightly different due to being a boundless quantity
    // Instead of being based on any absolute limits like pi and the size of the canvas, we instead make it so the scale is a relative multiplier to the parent's scale

    // mutation strength 0.0 means 2.0 mult, and strength 1.0 means 1.0 mult
    let mult = 2.0 - (1.0 - state.mutation_strength);
    var new_scale = parent.scale;
    if coinflip(rng) == 1 {
        new_scale *= mult;
    }
    else {
        new_scale /= mult;
    }

    // Hue rotation value
    // Again, like in rotation, it's a relative offset to the parent's value
    var new_hue = 0.0;
    if state.hue_enabled == 1 {
        let random_off = f32(coinflip(rng)) * ((next_f32(rng) * PI) * state.mutation_strength);
        let new_hue_angle = wrap_angle(radians(parent.hue) + random_off);
        new_hue = degrees(new_hue_angle);
    }

    return Candidate(parent.texture_id, new_pos.x, new_pos.y, new_ang, new_scale, new_hue);
}

// Random u32
fn next_u32(state: ptr<function,u32>) -> u32 {
    var x = *state;
    x = x ^ (x << 13);
    x = x ^ (x >> 7);
    x = x ^ (x << 17);
    // Update the new state but also return it as output
    *state = x;
    return x;
}

// Random f32 from 0.0 - 1.0
fn next_f32(state: ptr<function,u32>) -> f32 {
    return f32(next_u32(state)) / 4294967295.0;
}

// Random i32 of either 1 or -1
fn coinflip(state: ptr<function,u32>) -> i32 {
    let r = next_f32(state) >= 0.5;
    if r {
        return 1;
    }
    else {
        return -1;
    }
}

/// Linearly interpolates between two values
fn lerp(value1: f32, value2: f32, amount: f32) -> f32 {
    return value1 + (value2 - value1) * amount;
}

/// Returns a random point on a given x,y sized plane inclusively
fn random_point_2d(plane_size: vec2<u32>, rng: ptr<function,u32>) -> vec2<f32> {
    // Fun fact, [..=] is the "range inclusive" operator, so doing 0..=10 means from 0 to 10 inclusive
    let x = next_f32(rng) * f32(plane_size.x);
    let y = next_f32(rng) * f32(plane_size.y);
    return vec2<f32>(x, y);
}

/// Reduces a given angle to a value between -pi and pi
fn wrap_angle(input_angle: f32) -> f32 {
    var angle = input_angle;
    angle = (angle + PI) % TAU;
    if angle < 0.0 {
        angle += TAU;
    }
    return angle - PI;
}
