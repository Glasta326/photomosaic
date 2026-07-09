struct AtlasEntry {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rotated: u32,
};

struct Candidate {
    texture_id: u32,
    pos_x: f32,
    pos_y: f32,
    rotation: f32,
    scale: f32,
};

@group(0) @binding(0) var input_atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var<storage,read> input_atlas_entries: array<AtlasEntry>;
@group(0) @binding(2) var input_target_texture: texture_2d<f32>;
@group(0) @binding(3) var input_canvas_texture: texture_2d<f32>;

@group(0) @binding(4) var<storage,read> input_candidates: array<Candidate>;
@group(0) @binding(5) var<storage,read_write> output_score: array<f32>;

@compute @workgroup_size(64)
fn process(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_index = global_id.x;

    if thread_index > arrayLength(&output_score){
        output_score[thread_index] = 0.0; // If it's out of the image bounds then it shouldnt change the score
        return;
    }
    
    let canvas_texture_size = textureDimensions(input_canvas_texture);
    let this_candidate = input_candidates[thread_index];

    // Total color difference score for this candidate
    var score_sum = 0.0;

    // Iterate over every pixel in the canvas image and cal
    for (var y: u32 = 0; y < canvas_texture_size.y; y++) {
        for (var x: u32 = 0; x < canvas_texture_size.x; x++) {
            let coords = vec2<u32>(x, y);

            // Get the post-transform pixel from the atlas
            let candidate_pixel = get_atlas_pixel_from_candidate(this_candidate, coords);
            let canvas_pixel = textureLoad(input_canvas_texture, coords, 0);

            // Simulate drawing to the canvas
            let result = draw(candidate_pixel, canvas_pixel);

            // Calculate how close this result is to the target image
            let target_pixel = textureLoad(input_target_texture, coords, 0);
            let score = distance(result, target_pixel);

            score_sum += score;
        }
    }
    output_score[thread_index] = score_sum;
}

// Alpha-composite drawing function
// Essentially, mixes the colors roughly how you'd intuativley think they'd mix based on opacity
// A fully opaque source fully overrides the color below it
fn draw(src: vec4<f32>, dest: vec4<f32>) -> vec4<f32> {
    let r = (src.r * src.a) + (dest.r * (1.0 - src.a));
    let g = (src.g * src.a) + (dest.g * (1.0 - src.a));
    let b = (src.b * src.a) + (dest.b * (1.0 - src.a));
    let a = src.a + (dest.a * (1.0 - src.a));

    return vec4<f32>(r, g, b, a);
}

// TL;DR:
// Given a target position, applies the inverse of the candidate's transform to get the right pixel from the canvas texture
// 
// Long version:
// Each (x,y) position in our loop is responsible for one color difference calculation between our candidate image, and the canvas texture
// But the candidate itself is not a texture, so to get our source color, we need to ask the question "Which pixel of the atlas should i read?"
// because our candidate already provides us with the transforms that get applied to the texture, we can almost visualise it as a math equation
// (a,b) + offset(123,456) + rotate(0.344) * scale(0.85) = (x,y)
// Where (x,y) is the position in the loop that iterates over every pixel
// So, we can just re-arrange the equation to get the inverse operations needed to calculate (a,b) inside the atlas
// (x,y) - offset(123,456) + rotate(-0.344) / scale(0.85) = (a,b)
// In the actual implementation, there is a bit more nuance with moving the origin to the center of the texture and ect
fn get_atlas_pixel_from_candidate(candidate: Candidate, coords: vec2<u32>) -> vec4<f32> {
    let atlas_region = input_atlas_entries[candidate.texture_id];
    let output_pixel = vec2<f32>(coords);
    
    let candidate_texture_pos = vec2<f32>(candidate.pos_x, candidate.pos_y);
    
    // Move into texture-local space
    var p = output_pixel - candidate_texture_pos;

    // Inverse the rotation
    let c = cos(-candidate.rotation);
    let s = sin(-candidate.rotation);
    p = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);

    // Inverse the scaling
    p /= candidate.scale;

    // Move origin back to top-left
    let half = vec2<f32>(f32(atlas_region.width), f32(atlas_region.height)) * 0.5;
    p += half;

    // Ensure we are inside the atlas region for this candidate
    // Dont forget it's texture-local so (0,0) is the top-left of the texture inside the atlas
    if p.x < 0.0 ||
        p.y < 0.0 ||
        p.x >= f32(atlas_region.width) ||
        p.y >= f32(atlas_region.height) {
        return vec4<f32>(0.0);
    }

    // p is texture local space, so we add it to the absolute position of the region to get the coordinate of the pixel we want to sample
    let atlas_coord = vec2<u32>(atlas_region.x + u32(p.x), atlas_region.y + u32(p.y));

    return textureLoad(input_atlas_texture, atlas_coord, 0);
}
