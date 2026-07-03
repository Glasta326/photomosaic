struct AtlasEntry {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rotated: u32,
};

struct Candidate{
    texture_id: u32,
    x_offset: u32,
    y_offset: u32,
    scale: f32,
    rot: f32,
};

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<storage, read> input_regions: array<AtlasEntry>;

@compute @workgroup_size(16,16)
fn process(@builtin(global_invocation_id) global_id: vec3<u32>) {

    let sprite_pos = vec2<f32>(0.0, 0.0);
    let sprite_scale = 1.0;
    let sprite_rot = 0.0;
    let sprite_tex_id = 0; // We always just use whatever the first texture is while testing

    let output_pixel = vec2<f32>(global_id.xy);
    let region = input_regions[0];

    // Move to sprite-relative space
    var p = output_pixel - sprite_pos;

    // Move to the center 
    let half_size = vec2<f32>(f32(region.width), f32(region.height)) * 0.5;
    p -= half_size;

    // Inverse rotation
    let c = cos(-sprite_rot);
    let s = sin(-sprite_rot);
    p = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);

    // Inverse scale
    p /= sprite_scale;

    p += half_size;

    if p.x < 0.0 ||
        p.y < 0.0 ||
        p.x >= f32(region.width) ||
        p.y >= f32(region.height) {
        return;
    }

    let atlas_coord = vec2<i32>(
        i32(region.x) + i32(p.x),
        i32(region.y) + i32(p.y)
    );

    let color = textureLoad(
        input_texture,
        atlas_coord,
        0
    );

    textureStore(
        output_texture,
        vec2<i32>(global_id.xy),
        color
    );
    
}
