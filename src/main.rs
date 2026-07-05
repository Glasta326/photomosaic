use std::path::PathBuf;

use crate::candidate::Candidate;

mod candidate;
mod compute_system;
mod config_parse;
mod data_reader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If config parsing returns None, that means an early-exit parameter like -v or --help was used, so we return before doing anything.
    let Some(cfg) = config_parse::parse()? else {
        println!("Exiting...");
        return Ok(());
    };
    println!("{}", cfg.display());

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

    let compute = compute_system::compute::Compute::new_init(
        &cfg,
        atlas_texture,
        atlas_entries,
        target_texture,
    )?;

    let x = compute.run(&cfg)?;
    println!("{:#?}", x);
    let x = compute.run(&cfg)?;
    println!("{:#?}", x);
    
    return Ok(());
}
