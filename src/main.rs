use image::imageops;
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::utils::math_utils::{self, lerp};

mod candidate;
mod compute_system;
mod config_parse;
mod data_reader;
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

    // let compute = compute_system::compute::Compute::new_init(
    //     &cfg,
    //     atlas_texture,
    //     atlas_entries,
    //     target_texture,
    // )?;

    // let x = compute.run(&cfg)?;
    // println!("{:#?}", x);
    // let x = compute.run(&cfg)?;
    // println!("{:#?}", x);
    cfg.mutation_strength = 1.0;
    let mut c = candidate::Candidate::new(0, 100, 100, 0.0, 1.0);

    for i in 0..1000 {
        c = c.mutate_new(&cfg, &mut rng);
        println!("{}", c);
    }



    return Ok(());
}
