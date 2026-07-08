use std::f32::consts::{PI, TAU};

use image::{Luma, RgbaImage, imageops};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    candidate::Candidate,
    gpu::draw_shader,
    utils::math_utils::{self, lerp},
};

mod candidate;
mod config_parse;
mod data_reader;
mod gpu;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If config parsing returns None, that means an early-exit parameter like -v or --help was used, so we return before doing anything.
    let Some(mut cfg) = config_parse::parse()? else {
        println!("Exiting...");
        return Ok(());
    };
    println!("{}", cfg.display());

    // Create the rng from the config seed
    let mut rng = StdRng::seed_from_u64(cfg.seed);

    let (atlas_texture, atlas_entries) = data_reader::read_atlas_data(&cfg)?;
    println!(
        "Loaded {} by {} texture atlas with {} entries",
        atlas_texture.width(),
        atlas_texture.height(),
        atlas_entries.len()
    );

    let target_texture = data_reader::read_target_texture(&cfg)?;
    println!(
        "Loaded {} by {} target texture: [{}]",
        target_texture.width(),
        target_texture.height(),
        cfg.target_texture.file_name().unwrap().display()
    );

    // Update cfg data
    cfg.extra_data.atlas_dimensions = atlas_texture.dimensions();
    cfg.extra_data.target_dimensions = target_texture.dimensions();

    let context = gpu::GpuContext::init(&cfg, atlas_texture, atlas_entries, target_texture)?;
    let score_shader = gpu::ScoreShader::init(&cfg, &context)?;
    let draw_shader = gpu::DrawShader::init(&cfg, &context)?;

    // test color difference result on 2 candiates
    // expected result is to see results differ very slightly
    let mut candidates: Vec<Candidate> = vec![];
    candidates.push(Candidate::new(4, 200, 200, 0.0, 1.0));
    candidates.push(Candidate::new(2, 200, 200, 0.0, 1.0));

    
    for i in 0..=9 {
        draw_shader.run(&context, &Candidate::new(i, rng.random_range(0..1280), rng.random_range(0..720), rng.random_range(0.0..TAU), 1.0))?;
    }
    
    let x = score_shader.run(&cfg, &context, &candidates)?;
    println!("{:#?}",x);


    
    return Ok(());
}
