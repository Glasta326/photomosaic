struct AtlasEntry {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rotated: u32,
};

struct Candidate {
    texture_id: u32,
    pos_x: u32,
    pos_y: u32,
    rotation: f32,
    scale: f32,
};

@group(0) @binding(0) var input_atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var<storage,read> input_atlas_entries: array<AtlasEntry>;
@group(0) @binding(2) var input_target_texture: texture_2d<f32>;
@group(0) @binding(3) var input_canvas_texture: texture_2d<f32>;

@group(0) @binding(4) var<uniform> input_candidate: Candidate;
@group(0) @binding(5) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16,16)
fn process(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<u32>(global_id.xy);
    
    let candidate_pixel = get_atlas_pixel_from_candidate(input_candidate, coords);
    let canvas_pixel = textureLoad(input_canvas_texture, coords, 0);
    let result = draw(candidate_pixel, canvas_pixel);
    
    textureStore(output_texture, coords, result);
}

// See score_shader.wgsl for details
fn draw(src: vec4<f32>, dest: vec4<f32>) -> vec4<f32> {
    let r = (src.r * src.a) + (dest.r * (1.0 - src.a));
    let g = (src.g * src.a) + (dest.g * (1.0 - src.a));
    let b = (src.b * src.a) + (dest.b * (1.0 - src.a));
    let a = src.a + (dest.a * (1.0 - src.a));

    return vec4<f32>(r, g, b, a);
}

// See score_shader.wgsl for details
fn get_atlas_pixel_from_candidate(candidate: Candidate, coords: vec2<u32>) -> vec4<f32> {
    let atlas_region = input_atlas_entries[candidate.texture_id];
    let output_pixel = vec2<f32>(coords);
    
    let candidate_texture_pos = vec2<f32>(f32(candidate.pos_x), f32(candidate.pos_y));
    
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
