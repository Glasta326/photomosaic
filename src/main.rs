use image::{Luma, RgbaImage, imageops};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    candidate::Candidate,
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
    cfg.candidates_per_generation = 921600;
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

    // Create blank canvas texture
    let canvas_texture = RgbaImage::new(
        cfg.extra_data.target_dimensions.0,
        cfg.extra_data.target_dimensions.1,
    );

    let context = gpu::GpuContext::init(atlas_texture, atlas_entries, target_texture)?;
    let score_shader = gpu::ScoreShader::init(&cfg, &context)?;

    let mut candidates: Vec<Candidate> = vec![];
    candidates.push(Candidate::new(0, 0, 0, 0.0, 1.0));
    

    let x = score_shader.run(&cfg, &context, &candidates, &canvas_texture)?;
    println!("{}",x[0]);

    let width = 1280;
    let height = 720;
    let values: Vec<f32> = x;

    let mut img = image::GrayImage::new(width, height);

    for (pixel, value) in img.pixels_mut().zip(values.iter()) {
        let gray = (value.clamp(0.0, 1.0) * 255.0) as u8;
        *pixel = Luma([gray]);
    }

    img.save("debug/testing_output/image.png")?;

    return Ok(());
}
