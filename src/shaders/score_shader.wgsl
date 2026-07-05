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
@group(0) @binding(4) var<storage,read> input_candidates: array<Candidate>;
@group(0) @binding(5) var<storage,read_write> output_score: array<f32>;

@compute @workgroup_size(64)
fn process(@builtin(global_invocation_id) global_id: vec3<u32>) {


    let index = global_id.x;

    if index >= arrayLength(&output_score){
        return;
    }

    output_score[index] = 125.04;
}
