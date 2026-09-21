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
    hue: f32,
};

@group(0) @binding(0) var input_atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var<storage,read> input_atlas_entries: array<AtlasEntry>;
@group(0) @binding(2) var input_target_texture: texture_2d<f32>;
@group(0) @binding(3) var input_canvas_texture: texture_2d<f32>;

@group(0) @binding(4) var<storage,read> input_candidates: array<Candidate>;
// Say NO to pesky gpu manufactures limiting your buffer size!
// With this simple trick, you can quadruple your max buffer size!!
@group(0) @binding(5) var<storage,read_write> internal_candidate_pixel_scores_0: array<f32>;
@group(0) @binding(6) var<storage,read_write> internal_candidate_pixel_scores_1: array<f32>;
@group(0) @binding(7) var<storage,read_write> internal_candidate_pixel_scores_2: array<f32>;
@group(0) @binding(8) var<storage,read_write> internal_candidate_pixel_scores_3: array<f32>;
@group(0) @binding(9) var<storage,read_write> output_score: array<f32>;

@compute @workgroup_size(4,8,8)
fn score_3D(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let candidate_index = global_id.x;
    let p_x = global_id.y;
    let p_y = global_id.z;
    let canvas_texture_size = textureDimensions(input_canvas_texture);

    let candidates_per_buffer = u32(ceil(f32(arrayLength(&input_candidates)) / 4.0)); // I believe standard u32 division ceil's by default but im paranoid
    let buffer_index = u32(floor(f32(candidate_index) / f32(candidates_per_buffer)));
    let local_candidate_index = candidate_index % candidates_per_buffer;

    // Sometimes more threads get allocated than the number of candidates we actually have, so just end early if we're an excess thread
    if candidate_index >= arrayLength(&input_candidates) {
        return;
    }
    // Ditto for the shader's y and z dimensions 
    if p_x >= canvas_texture_size.x || p_y >= canvas_texture_size.y {
        return;
    }
    // NOTE: also dont worry about "but does that mean there's excess space in the candidate_pixel_scores buffer?
    // No, because we calculate it's size wayy before spinning up the shader or anything
    // if we let these excess threads attempt to place an output, it would actually be trying to use an OOB index

    let this_candidate = input_candidates[candidate_index];
    let coords = vec2<u32>(p_x, p_y);

    // Get the post-transform pixel from the atlas
    let candidate_pixel_base = get_atlas_pixel_from_candidate(this_candidate, coords);
    let candidate_pixel_hueshifted = shift_hue(candidate_pixel_base, this_candidate.hue);

    // Simulate drawing to the canvas
    let canvas_pixel = textureLoad(input_canvas_texture, coords, 0);
    let result = draw(candidate_pixel_hueshifted, canvas_pixel);

    // Calculate how close this result is to the target image
    let target_pixel = textureLoad(input_target_texture, coords, 0);
    let score = distance(result, target_pixel);

    // store output score for this candidate pixel
    let pixel_index = p_y * canvas_texture_size.x + p_x;
    let output_index = local_candidate_index * (canvas_texture_size.x * canvas_texture_size.y) + pixel_index;

    if buffer_index == 0 {
        internal_candidate_pixel_scores_0[output_index] = score;
    }
    if buffer_index == 1 {
        internal_candidate_pixel_scores_1[output_index] = score;
    }
    if buffer_index == 2 {
        internal_candidate_pixel_scores_2[output_index] = score;
    }
    if buffer_index == 3 {
        internal_candidate_pixel_scores_3[output_index] = score;
    }
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

    // If this texture is rotated inside the atlas we need to account for that
    var sample_coord = p;
    if atlas_region.rotated == 1u {
        sample_coord = vec2<f32>(
            f32(atlas_region.height) - 1.0 - p.y,
            p.x
        );
    }

    // p is texture local space, so we add it to the absolute position of the region to get the coordinate of the pixel we want to sample
    let atlas_coord = vec2<u32>(atlas_region.x + u32(sample_coord.x), atlas_region.y + u32(sample_coord.y));

    return textureLoad(input_atlas_texture, atlas_coord, 0);
}

fn shift_hue(rgba: vec4<f32>, hue_shift: f32) -> vec4<f32> {
    let rgb = rgba.rgb;

    let max_c = max(max(rgb.r, rgb.g), rgb.b);
    let min_c = min(min(rgb.r, rgb.g), rgb.b);
    let delta = max_c - min_c;

    var hue = 0.0;

    if delta != 0.0 {
        if max_c == rgb.r {
            hue = 60.0 * ((rgb.g - rgb.b) / delta);

            if hue < 0.0 {
                hue += 360.0;
            }
        } else if max_c == rgb.g {
            hue = 60.0 * ((rgb.b - rgb.r) / delta + 2.0);
        } else {
            hue = 60.0 * ((rgb.r - rgb.g) / delta + 4.0);
        }
    }

    // Apply hue shift and wrap into [0, 360)
    hue = hue + hue_shift;
    hue = hue - floor(hue / 360.0) * 360.0;

    // Convert HSV back to RGB.
    let saturation = select(0.0, delta / max_c, max_c != 0.0);
    let value = max_c;

    let chroma = value * saturation;
    let h = hue / 60.0;
    let x = chroma * (1.0 - abs((h % 2.0) - 1.0));

    var rgb_prime = vec3<f32>(0.0);

    if h < 1.0 {
        rgb_prime = vec3<f32>(chroma, x, 0.0);
    } else if h < 2.0 {
        rgb_prime = vec3<f32>(x, chroma, 0.0);
    } else if h < 3.0 {
        rgb_prime = vec3<f32>(0.0, chroma, x);
    } else if h < 4.0 {
        rgb_prime = vec3<f32>(0.0, x, chroma);
    } else if h < 5.0 {
        rgb_prime = vec3<f32>(x, 0.0, chroma);
    } else {
        rgb_prime = vec3<f32>(chroma, 0.0, x);
    }

    let m = value - chroma;
    let result_rgb = rgb_prime + vec3<f32>(m);

    return vec4<f32>(result_rgb, rgba.a);
}

// Explanation for myself:
// So the way shaders are structured, each workgroup contains a number of threads, so you essentially have:
// for workgroup in 0..10
// {
//     for thread in 0..256
//     {
//         run_shader();
//     }
// }
// so in our case, we assign one workgroup for each candidate, and 256 threads for each workgroup
// so our params: workgroup_id , local_id;
// just mean: "id of this workgroup(ranges from 0 - Candidate count)", "id of this thread inside the workgroup(ranges from 0-256 because we set it at that)"
const workgroup_size: u32 = 256;
var<workgroup> thread_sums: array<f32,256>;
@compute @workgroup_size(workgroup_size)
fn reduce(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let canvas_texture_size = textureDimensions(input_canvas_texture);
    let pixel_count = canvas_texture_size.x * canvas_texture_size.y;

    let this_candidate_index = workgroup_id.x;
    let this_thread_index = local_id.x;

    let candidates_per_buffer = u32(ceil(f32(arrayLength(&input_candidates)) / 4.0)); // I believe standard u32 division ceil's by default but im paranoid
    let buffer_index = u32(floor(f32(this_candidate_index) / f32(candidates_per_buffer)));
    let local_candidate_index = this_candidate_index % candidates_per_buffer;

    // The giant array of data points is essentially split like:
    // [candidate01 data..., candidate02 data..., ect..]
    // so we need to move into our candidate's block of memory
    // each candidate's block of memory contains as many values as there are pixels
    let this_candidate_data_entry_offset = local_candidate_index * pixel_count;

    // And because each candidate data "block" is pixel_count wide, we need to limit accesses to this range:
    let this_candidate_data_limit = this_candidate_data_entry_offset + pixel_count;
    // Otherwise we're getting values from the next candidate over

    // There's almost always going to be more than 256 values in our candidate's value block however, but we only have 256 threads to work with
    // So first, we need to condense the values array down to 256 values
    // We do this by summing each 256'th value in the array
    var this_thread_sum = 0.0;
    for (var i = this_thread_index; i < pixel_count; i += workgroup_size) {
        let index = this_candidate_data_entry_offset + i;

        if buffer_index == 0 {
            this_thread_sum += internal_candidate_pixel_scores_0[index];
        }
        if buffer_index == 1 {
            this_thread_sum += internal_candidate_pixel_scores_1[index];
        }
        if buffer_index == 2 {
            this_thread_sum += internal_candidate_pixel_scores_2[index];
        }
        if buffer_index == 3 {
            this_thread_sum += internal_candidate_pixel_scores_3[index];
        }
    }

    // For a more visual explanation, suppose we only have 5 threads, and the image is 4x4 so we have 16 values per candidate
    // internal_data -> [candidate 1 stuff.., candidate 2 stuff, this candidate stuff, candidate 4 stuf..., ...]
    // (again it's just a big list of numbers, but effectivley it is split this way)
    // this candidate stuff -> [4,6,2,1,2,3,5,2,1,2, 3, 2, 1, 2, 3, 5]
    // 's indexes ->           [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
    // 
    // thread 0 will sum the values at index 0,5,10,15
    // thread 1 will sum indexes             1,6,11,16
    // thread 2:                             2,7,12
    // thread 3:                             3,8,13
    // thread 4:                             4,9,14

    // now each thread has it's own sum , we store that in the thread_sums array, which is a per-workgroup array for this shader
    // so like, workgroup 1 gets its own thread_sums array, workgroup 2 gets its own, ect
    thread_sums[this_thread_index] = this_thread_sum;

    // Now untill this point, each thread has been just doing its own thing,
    // but we need to wait untill every thread has calculated it's sum and put it into it's index in thread_sums[].
    // this function makes all threads wait untill theyre all caught up with eachother and at the exact same place
    workgroupBarrier();

    // Now comes the actual parralel reduction bit.
    // Simply, threads 0 - 128 will grab two values from the array, add them, and store the result in their indexes
    // then this is repeated with threads 0 - 64, and all the way down to one
    // so for example, if there were 4 threads total, and the thread_sums were [25,50,10,40]
    // thread 0 would grab 25 and 10, and store 35 in thread_sums[0]
    // thread 1 grabs 50,40 and stores 90 in thread sums [1]
    // and then on the next loop, only thread 0 runs, grabs 35 and 90, and stores 125 in thread_sums[0]
    // and then the result is complete
    var threads_remaining = u32(workgroup_size / 2);
    while threads_remaining > 0 {
        if this_thread_index < threads_remaining {
            thread_sums[this_thread_index] += thread_sums[this_thread_index + threads_remaining];
        }

        // Because we're working on the shared thread_sums array, we need to keep all the threads syncronised, so again,
        // we tell them to wait untill every thread is ready, and then proceed
        workgroupBarrier();
        threads_remaining /= u32(2);
    }

    // After everything, thread 0 stores it's result
    if this_thread_index == 0 {
        output_score[this_candidate_index] = thread_sums[0];
    }
}
